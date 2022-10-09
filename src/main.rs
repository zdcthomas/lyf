use std::{
    collections::HashMap,
    fmt::Display,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
    vec,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::disable_raw_mode,
};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Points, Rectangle},
        Axis, Block, BorderType, Borders, Chart, Dataset, Paragraph,
    },
};

#[derive(PartialEq, Eq, Clone)]
enum CellState {
    Alive,
    Dead,
}

#[derive(Clone)]
struct Cell {
    state: CellState,
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rep = match self.state {
            CellState::Alive => "█",
            CellState::Dead => " ",
        };
        write!(f, "{}", rep)
    }
}

type Coord = (i32, i32);
type BoardInner = HashMap<Coord, Cell>;

struct Board {
    _inner: BoardInner,
}

const DEAD_CELL: Cell = Cell {
    state: CellState::Dead,
};

impl Board {
    /// Creates a new [`Board`].
    fn new(_inner: BoardInner) -> Self {
        Self { _inner }
    }

    fn draw(&self, x_window: i32, y_window: i32) -> Result<()> {
        clearscreen::clear().context("Tried to clear screen")?;
        for y in 0..y_window {
            for x in 0..x_window {
                let cell = self.get((x, y));
                print!("{cell}");
            }
            print!("\n");
        }

        Ok(())
    }

    fn spawn(&mut self, (x, y): Coord) {
        if let Some(cell) = self._inner.get_mut(&(x, y)) {
            cell.state = CellState::Alive;
        } else {
            self._inner.insert(
                (x, y),
                Cell {
                    state: CellState::Alive,
                },
            );
        }
    }

    fn get(&self, coord: Coord) -> &Cell {
        if let Some(cell) = self._inner.get(&coord) {
            cell
        } else {
            &DEAD_CELL
        }
    }

    fn get_neighbors(&self, (x, y): Coord) -> [&Cell; 8] {
        [
            self.get((x - 1, y)),
            self.get((x + 1, y)),
            self.get((x, y - 1)),
            self.get((x, y + 1)),
            self.get((x + 1, y + 1)),
            self.get((x + 1, y - 1)),
            self.get((x - 1, y + 1)),
            self.get((x - 1, y - 1)),
        ]
    }

    fn number_of_living_neighbors(&self, coord: Coord) -> u8 {
        let neighbors = self.get_neighbors(coord);
        let mut count = 0;
        for ele in neighbors {
            if ele.state == CellState::Alive {
                count += 1;
            }
        }
        count
    }

    fn surrounding((x, y): Coord) -> [Coord; 8] {
        [
            (x - 1, y),
            (x + 1, y),
            (x, y - 1),
            (x, y + 1),
            (x + 1, y + 1),
            (x + 1, y - 1),
            (x - 1, y + 1),
            (x - 1, y - 1),
        ]
    }

    fn get_dead_neighbors(&self, coord: Coord) -> Vec<Coord> {
        let mut deads: Vec<Coord> = vec![];
        for ele in Self::surrounding(coord) {
            if self.get(ele).state == CellState::Dead {
                deads.push(ele);
            }
        }
        deads
    }

    fn make_glider(&mut self, (x, y): Coord) {
        for ele in [
            (x, y),
            (x + 1, y + 1),
            (x - 1, y + 2),
            (x, y + 2),
            (x + 1, y + 2),
        ] {
            self.spawn(ele);
        }
    }

    fn points(&self) -> Vec<(f64, f64)> {
        self._inner
            .iter()
            .filter(|(_, cell)| matches!(cell.state, CellState::Alive))
            .map(|((x, y), _cell)| (*x as f64, *y as f64))
            .collect()
    }

    fn progess(self) -> Self {
        /*
        Any live cell with fewer than two live neighbours dies, as if by underpopulation.           X
        Any live cell with two or three live neighbours lives on to the next generation.            X
        Any live cell with more than three live neighbours dies, as if by overpopulation.           X
        Any dead cell with exactly three live neighbours becomes a live cell, as if by reproduction.
        */
        let mut new_board = Board::new(HashMap::new());

        for (coord, cell) in self._inner.iter() {
            let living_neighbors = self.number_of_living_neighbors(*coord);
            match cell.state {
                CellState::Alive => {
                    if living_neighbors == 2 || living_neighbors == 3 {
                        new_board.spawn(*coord);
                    };

                    for dead_cell_coord in self.get_dead_neighbors(*coord) {
                        if self.number_of_living_neighbors(dead_cell_coord) == 3 {
                            new_board.spawn(dead_cell_coord);
                        }
                    }

                    /*
                    1. get all dead neighbors
                    2. number_of_living_neighbors for each dead neighbor
                        2 a. if 3, spawn cell at that coord
                    */
                }
                CellState::Dead => {
                    // if living_neighbors == 3 {
                    //     new_board.spawn(*coord)
                    // }
                }
            }
        }
        new_board
    }
}

mod disp {
    type Term = Terminal<CrosstermBackend<Stdout>>;

    use anyhow::{Context, Result};
    use crossterm::terminal::enable_raw_mode;
    use tui::{backend::CrosstermBackend, Terminal};

    use std::io::{self, Stdout};

    pub fn setup_terminal() -> Result<Term> {
        enable_raw_mode()?;
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        Terminal::new(backend).context("Setting up terminal")
    }
}
enum UserEvent<InputType> {
    Input(InputType),
    Tick,
}

struct App {
    paused: bool,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
}

impl App {
    fn new() -> Self {
        Self {
            paused: false,
            x_bounds: [-50.0, 50.0],
            y_bounds: [-50.0, 50.0],
        }
    }

    fn translate(&mut self, x_mov: f64, y_mov: f64) {
        self.x_bounds[0] += x_mov;
        self.x_bounds[1] += x_mov;

        self.y_bounds[0] += y_mov;
        self.y_bounds[1] += y_mov;
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused
    }

    fn expand_frame_x(&mut self) {
        self.x_bounds[0] -= 5.0;
        self.x_bounds[1] += 5.0;
    }

    fn expand_frame_y(&mut self) {
        self.y_bounds[0] -= 5.0;
        self.y_bounds[1] += 5.0;
    }

    fn contract_frame_y(&mut self) {
        self.y_bounds[0] += 5.0;
        self.y_bounds[1] -= 5.0;
    }

    fn contract_frame_x(&mut self) {
        self.x_bounds[0] += 5.0;
        self.x_bounds[1] -= 5.0;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut term = disp::setup_terminal()?;

    let mut board = Board::new(HashMap::new());
    board.make_glider((5, 5));

    board.spawn((10, 10));
    board.spawn((10, 11));
    board.spawn((10, 12));

    term.clear()?;

    let mut app = App::new();

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    // sets up input loop
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let Event::Key(key) = event::read().expect("Problem with reading events") {
                    tx.send(UserEvent::Input(key))
                        .expect("uh oh, couldn't send event");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(UserEvent::Tick) {
                    last_tick = Instant::now()
                }
            }
        }
    });

    loop {
        term.draw(|rect| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Length(3), Constraint::Min(2)].as_ref())
                .split(rect.size());

            let title = Paragraph::new("q/esc: quit, spc: stop, start, hjkl/←↓↑→: move view port, HJKL: epxand/contract view")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title("Controls")
                        .border_type(BorderType::Plain),
                );

            let canvas = Canvas::default()
                .block(Block::default().borders(Borders::ALL).title("Game of life"))
                .paint(|ctx| {
                    ctx.draw(&Points {
                        coords: &board.points(),
                        color: Color::Yellow,
                    });
                })
                .x_bounds(app.x_bounds)
                .y_bounds(app.y_bounds);
            rect.render_widget(title, chunks[0]);
            rect.render_widget(canvas, chunks[1]);
        })?;

        match rx.recv()? {
            UserEvent::Input(event) => match event.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    disable_raw_mode()?;
                    clearscreen::clear().context("Tried to clear screen")?;
                    term.show_cursor()?;
                    break Ok(());
                }
                KeyCode::Char(' ') => app.toggle_pause(),
                KeyCode::Char('h') | KeyCode::Left => app.translate(-10.0, 0.0),
                KeyCode::Char('l') | KeyCode::Right => app.translate(10.0, 0.0),
                KeyCode::Char('k') | KeyCode::Up => app.translate(0.0, 10.0),
                KeyCode::Char('j') | KeyCode::Down => app.translate(0.0, -10.0),

                KeyCode::Char('H') => app.contract_frame_x(),
                KeyCode::Char('L') => app.expand_frame_x(),
                KeyCode::Char('K') => app.expand_frame_y(),
                KeyCode::Char('J') => app.contract_frame_y(),
                _ => {}
            },
            UserEvent::Tick => {
                if !app.paused {
                    board = board.progess();
                }
            }
        };
    }
}
