use std::{collections::HashMap, fmt::Display, thread, time};

use anyhow::{Context, Ok, Result};

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

    fn progess(self) -> Board {
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

fn main() {
    let mut board = Board::new(HashMap::new());
    board.make_glider((5, 5));

    board.spawn((10, 10));
    board.spawn((10, 11));
    board.spawn((10, 12));

    loop {
        board.draw(50, 50).unwrap();
        board = board.progess();
        thread::sleep(time::Duration::from_millis(100));
    }
}
