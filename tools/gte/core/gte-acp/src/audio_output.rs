use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::ops::IndexMut;
use dasp_graph::{Buffer, Input, NodeData};
use dasp_interpolate::linear::Linear;
use dasp_signal::Signal;
use log::{debug, error, trace, warn};
use rtrb::{Consumer, Producer, RingBuffer};
use petgraph::prelude::NodeIndex;

pub struct GameTankSignal {
    buffer: Consumer<u8>,
}

impl GameTankSignal {
    pub fn new(buffer: Consumer<u8>) -> Self {
        Self {
            buffer,
        }
    }
}

impl Signal for GameTankSignal {
    type Frame = f32;

    fn next(&mut self) -> Self::Frame {
        if let Ok(sample) = self.buffer.pop() {
            (sample as f32 / 255.0) * 2.0 - 1.0
        } else {
            warn!("FEED THE BUFFFEERRRRRR");
            0.0
        }
    }

    fn is_exhausted(&self) -> bool {
        self.buffer.slots() < 64
    }
}

pub struct GameTankAudio {
    pub producer: Producer<u8>,

    pub resampled: VecDeque<f32>,

    pub output_queue: Producer<Buffer>, // ring buffer for output buffers
    pub output_buffer: Consumer<Buffer>,

    pub sample_rate: f64,
    pub converter: Box<dyn Signal<Frame = f32> + Send>,
}

impl GameTankAudio {
    pub fn new(sample_rate: f64, target_sample_rate: f64) -> Self {
        // caps out around 48kHz, but technically the system can go higher...
        let (input_producer, input_buffer) = RingBuffer::<u8>::new(1024); // Ring buffer to hold GameTank samples
        let (output_producer, output_consumer) = RingBuffer::<Buffer>::new(4096); // Ring buffer to hold output buffers
        let interp = Linear::new(0.0, 0.0);

        let signal = GameTankSignal::new(input_buffer);
        let converter = signal.from_hz_to_hz(interp, sample_rate, target_sample_rate);

        Self {
            producer: input_producer,
            resampled: VecDeque::with_capacity(1024),
            output_queue: output_producer,
            output_buffer: output_consumer,
            sample_rate,
            converter: Box::new(converter),
        }
    }

    pub fn convert_to_output_buffers(&mut self) {
        while !self.converter.is_exhausted() {
            self.resampled.push_back(self.converter.next());
        }

        while self.resampled.len() >= 64 && self.output_queue.slots() >= 8 {
            if let Ok(chunk) = self.resampled.drain(..64).collect::<Vec<_>>().try_into() {
                let mut buf = Buffer::SILENT;
                for (b, v) in buf.iter_mut().zip::<[f32;64]>(chunk) {
                    *b = v;
                }
                self.output_queue.push(buf).unwrap()
            }
        }
    }
}
