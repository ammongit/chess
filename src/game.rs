/*
 * game.rs
 *
 * mallard-chess - Chess engine wrapper for fun
 * Copyright (C) 2022 Ammon Smith
 *
 * mallard-chess is available free of charge under the terms of the MIT
 * License. You are free to redistribute and/or modify it under those
 * terms. It is distributed in the hopes that it will be useful, but
 * WITHOUT ANY WARRANTY. See the LICENSE file for more details.
 */

use crate::config::Configuration;
use crate::engine::Engine;
use crate::stockfish::Stockfish;
use chess::{Board, MoveGen};
use std::fmt::Display;
use std::io::{self, BufRead, Stdin};
use std::str::FromStr;
use vampirc_uci::{parse_one, UciMessage};

#[derive(Debug)]
pub struct Game {
    pub board: Board,
    pub stockfish: Stockfish,
    input: Stdin,
    input_buffer: String,
}

impl Game {
    // Constructor
    pub fn new(config: &Configuration) -> Self {
        Game {
            board: Board::default(),
            stockfish: Stockfish::spawn(config.stockfish_nodes),
            input: io::stdin(),
            input_buffer: String::new(),
        }
    }

    // Communication
    fn receive(&mut self) -> UciMessage {
        self.input_buffer.clear();
        self.input
            .lock()
            .read_line(&mut self.input_buffer)
            .expect("Unable to read from stdin");

        parse_one(&self.input_buffer)
    }

    fn send<D: Display>(&mut self, command: D) {
        println!("{}", command);
    }

    // Execution
    pub fn main_loop(&mut self, engine: &dyn Engine) {
        loop {
            match self.receive() {
                // Set board position
                UciMessage::Position {
                    startpos,
                    fen,
                    moves,
                } => {
                    match (startpos, fen) {
                        (true, None) => self.board = Board::default(),
                        (false, Some(fen)) => {
                            self.board = Board::from_str(&fen.0).expect("Invalid FEN from server")
                        }
                        _ => panic!("Inconsistent startpos / fen in UciMessage::Position"),
                    }

                    for m in moves {
                        self.board = self.board.make_move_new(m);
                    }
                }

                // Request move decision from engine
                UciMessage::Go { .. } => self.decide_move(engine),

                // Status messages
                UciMessage::Uci => {
                    self.send(UciMessage::UciOk);
                    self.send(UciMessage::Id {
                        name: Some(format!("{} ({:?})", env!("CARGO_PKG_NAME"), engine.kind())),
                        author: None,
                    });
                    self.send(UciMessage::Id {
                        name: None,
                        author: Some(env!("CARGO_PKG_AUTHORS").into()),
                    });
                }

                UciMessage::ReadyOk => self.send(UciMessage::IsReady),

                // Terminal messages
                UciMessage::Quit => break,

                // Ignore unknown or unexpected messages
                _ => (),
            }
        }
    }

    fn decide_move(&mut self, engine: &dyn Engine) {
        let chosen_move = engine.choose_move(self);
        self.board = self.board.make_move_new(chosen_move);
        self.send(UciMessage::BestMove {
            best_move: chosen_move,
            ponder: None,
        });
    }

    // Methods
    #[inline]
    pub fn moves(&self) -> MoveGen {
        MoveGen::new_legal(&self.board)
    }
}
