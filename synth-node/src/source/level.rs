use dasp_graph::{Buffer, Input, Node};

use std::sync::mpsc::{self, Receiver, Sender};

pub enum LevelCommand {
    DeltaLevel(f32),
    SetLevel(f32),
}

pub struct Level {
    level: f32,
    rx: Option<Receiver<LevelCommand>>,
}

impl Level {
    pub fn new(val: f32) -> Self {
        Self {
            level: val,
            rx: None,
        }
    }

    pub fn with_channel(mut self) -> (Self, Sender<LevelCommand>) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        (self, tx)
    }

    fn process_command(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok(command) = rx.try_recv() {
                match command {
                    LevelCommand::DeltaLevel(delta) => {
                        self.level += delta;
                    }
                    LevelCommand::SetLevel(level) => {
                        self.level = level;
                    }
                }
            }
        }
    }
}

impl Node for Level {
    fn process(&mut self, _inputs: &[Input], output: &mut [Buffer]) {
        self.process_command();

        for i in 0..Buffer::LEN {
            for buffer in output.iter_mut() {
                buffer[i] = self.level;
            }
        }
    }
}
