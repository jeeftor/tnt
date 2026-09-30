//! Terminal interface and sampling loop for the TrueNAS sensor monitor.

mod sensors;

use std::collections::VecDeque;
use std::error::Error;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Sparkline};
use sensors::{Group, Reading, parse_sensors};

const HISTORY_LEN: usize = 120;

/// The latest reading and in-memory history for one sensor ID.
struct SensorState {
    reading: Reading,
    history: VecDeque<f64>,
    /// Highest value seen since this sensor first appeared in the current run.
    peak: f64,
}

impl SensorState {
    /// Start a sensor's history with its first reading.
    fn new(reading: Reading) -> Self {
        let value = reading.value;
        Self {
            reading,
            history: VecDeque::from([value]),
            peak: value,
        }
    }

    /// Record a new sample, retaining only the most recent `HISTORY_LEN` values.
    ///
    /// The peak covers the sensor's entire lifetime, even after older samples
    /// leave the history buffer.
    fn update(&mut self, reading: Reading) {
        self.peak = self.peak.max(reading.value);
        self.history.push_back(reading.value);
        if self.history.len() > HISTORY_LEN {
            self.history.pop_front();
        }
        self.reading = reading;
    }
}

/// Display state shared by the sampling and keyboard event loop.
struct App {
    sensors: Vec<SensorState>,
    selected: usize,
    list: ListState,
    status: String,
    last_sample: Option<Instant>,
    interval: Duration,
}

impl App {
    /// Create an empty display while waiting for the first sensor sample.
    fn new(interval: Duration) -> Self {
        Self {
            sensors: Vec::new(),
            selected: 0,
            list: ListState::default(),
            status: "Waiting for sensors…".into(),
            last_sample: None,
            interval,
        }
    }

    /// Replace the current sensor list while preserving history and selection by ID.
    ///
    /// Sensors missing from this sample lose their history. A returning sensor
    /// starts a new history and peak.
    fn update(&mut self, readings: Vec<Reading>) {
        let selected_id = self
            .sensors
            .get(self.selected)
            .map(|s| s.reading.id.clone());
        let mut old = std::mem::take(&mut self.sensors);
        self.sensors = readings
            .into_iter()
            .map(|reading| {
                if let Some(index) = old.iter().position(|s| s.reading.id == reading.id) {
                    let mut state = old.swap_remove(index);
                    state.update(reading);
                    state
                } else {
                    SensorState::new(reading)
                }
            })
            .collect();
        self.selected = selected_id
            .and_then(|id| self.sensors.iter().position(|s| s.reading.id == id))
            .unwrap_or(0);
        self.list
            .select((!self.sensors.is_empty()).then_some(self.selected));
        self.status = format!(
            "{} sensors · every {}s",
            self.sensors.len(),
            self.interval.as_secs()
        );
        self.last_sample = Some(Instant::now());
    }

    /// Move the selected row by one, stopping at the first or last sensor.
    fn move_selection(&mut self, direction: i32) {
        if self.sensors.is_empty() {
            return;
        }
        self.selected = if direction < 0 {
            self.selected.saturating_sub(1)
        } else {
            (self.selected + 1).min(self.sensors.len() - 1)
        };
        self.list.select(Some(self.selected));
    }
}

/// Sample `sensors -j` on a worker thread and send readings or errors to the UI.
///
/// This keeps command latency off the keyboard event loop. Each interval starts
/// after the previous command finishes; the command currently has no timeout.
fn start_collector(interval: Duration) -> Receiver<Result<Vec<Reading>, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            let result = Command::new("sensors")
                .arg("-j")
                .output()
                .map_err(|err| format!("sensors -j: {err}"))
                .and_then(|output| {
                    parse_sensors(&output.stdout).map_err(|err| {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        format!("Cannot parse sensors -j: {err} {stderr}")
                    })
                });
            if sender.send(result).is_err() {
                break;
            }
            std::thread::sleep(interval);
        }
    });
    receiver
}

/// Parse command-line options, initialize the terminal, and run the monitor.
fn main() -> Result<(), Box<dyn Error>> {
    let mut interval_secs = 5;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "tnt — TrueNAS Temps\n\nUsage: tnt [--interval SECONDS]\n\nKeys: ↑/↓ or j/k select, q quit"
                );
                return Ok(());
            }
            "--interval" => {
                let value = args.next().ok_or("--interval needs a value")?;
                interval_secs = value.parse::<u64>()?;
                if !(1..=60).contains(&interval_secs) {
                    return Err("--interval must be between 1 and 60 seconds".into());
                }
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    if !cfg!(target_os = "linux") {
        return Err("This first version reads Linux lm-sensors on TrueNAS".into());
    }
    let interval = Duration::from_secs(interval_secs);
    let receiver = start_collector(interval);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, receiver, interval);
    ratatui::restore();
    result
}

/// Process sensor results and keys, redrawing when state changes or age advances.
fn run(
    terminal: &mut DefaultTerminal,
    receiver: Receiver<Result<Vec<Reading>, String>>,
    interval: Duration,
) -> Result<(), Box<dyn Error>> {
    let mut app = App::new(interval);
    let mut dirty = true;
    let mut last_draw = Instant::now();
    loop {
        while let Ok(message) = receiver.try_recv() {
            match message {
                Ok(readings) => app.update(readings),
                Err(error) => app.status = error,
            }
            dirty = true;
        }
        if dirty || last_draw.elapsed() >= Duration::from_secs(1) {
            terminal.draw(|frame| draw(frame, &mut app))?;
            last_draw = Instant::now();
            dirty = false;
        }
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Up | KeyCode::Char('k') => {
                    app.move_selection(-1);
                    dirty = true;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.move_selection(1);
                    dirty = true;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Render the header, selected sensor history, sensor list, and status footer.
fn draw(frame: &mut Frame, app: &mut App) {
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(9),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(frame.area());

    let header = Line::from(vec![
        Span::styled(
            " TNT ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  TrueNAS Temps"),
    ]);
    frame.render_widget(Paragraph::new(header), parts[0]);

    if let Some(sensor) = app.sensors.get(app.selected) {
        let value = &sensor.reading;
        let ceiling = value.limit.unwrap_or(sensor.peak * 1.2).max(1.0).ceil() as u64;
        let samples: Vec<u64> = sensor
            .history
            .iter()
            .map(|v| v.max(0.0).round() as u64)
            .collect();
        let title = format!(
            " {} · {:.1} {} · peak {:.1} {}{} ",
            value.name,
            value.value,
            value.unit,
            sensor.peak,
            value.unit,
            value
                .limit
                .map(|n| format!(" · limit {n:.0}"))
                .unwrap_or_default()
        );
        let chart = Sparkline::default()
            .block(Block::default().title(title).borders(Borders::ALL))
            .data(&samples)
            .max(ceiling)
            .style(color_for(value));
        frame.render_widget(chart, parts[1]);
    } else {
        frame.render_widget(
            Paragraph::new("Waiting for readings from sensors -j")
                .block(Block::default().title(" History ").borders(Borders::ALL)),
            parts[1],
        );
    }

    let rows: Vec<ListItem<'_>> = app
        .sensors
        .iter()
        .map(|sensor| {
            let reading = &sensor.reading;
            let line = format!(
                " {:<7} {:<24.24} {:>6.1} {:<3}  {}",
                reading.group.name(),
                reading.name,
                reading.value,
                reading.unit,
                mini_history(&sensor.history, 24)
            );
            ListItem::new(line).style(Style::default().fg(color_for(reading)))
        })
        .collect();
    let list = List::new(rows)
        .block(
            Block::default()
                .title(" Sensors · current and history ")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("›");
    frame.render_stateful_widget(list, parts[2], &mut app.list);

    let age = app
        .last_sample
        .map(|last| format!(" · updated {}s ago", last.elapsed().as_secs()))
        .unwrap_or_default();
    let footer = format!(" {}{}  ·  ↑/↓ select  ·  q quit", app.status, age);
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().fg(Color::Gray)),
        parts[3],
    );
}

/// Choose a temperature warning color from its limit; fans remain cyan.
fn color_for(reading: &Reading) -> Color {
    if reading.group == Group::Fans {
        return Color::Cyan;
    }
    if let Some(limit) = reading.limit {
        if reading.value >= limit {
            return Color::Red;
        }
        if reading.value >= limit * 0.85 {
            return Color::Yellow;
        }
    }
    Color::Green
}

/// Draw recent values as blocks scaled between their own minimum and maximum.
///
/// This small row chart uses relative values, unlike the larger chart, which
/// uses the sensor's reported limit or observed peak as its ceiling.
fn mini_history(history: &VecDeque<f64>, width: usize) -> String {
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let recent: Vec<f64> = history.iter().rev().take(width).copied().collect();
    let low = recent.iter().copied().fold(f64::INFINITY, f64::min);
    let high = recent.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    recent
        .iter()
        .rev()
        .map(|value| {
            let level = if high > low {
                ((value - low) * 7.0 / (high - low)).round() as usize
            } else {
                3
            };
            BLOCKS[level.min(7)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::{App, draw, sensors::parse_sensors};

    #[test]
    fn renders_live_fan_and_temperature_rows() {
        let readings = parse_sensors(
            br#"{"dell_smm":{"Processor Fan":{"fan1_input":933},"Ambient":{"temp1_input":27}}}"#,
        )
        .unwrap();
        let mut app = App::new(Duration::from_secs(5));
        app.update(readings);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("TNT"));
        assert!(text.contains("Processor Fan"));
        assert!(text.contains("Ambient"));
    }
}
