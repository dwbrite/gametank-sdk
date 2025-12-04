use dasp_graph::{Buffer, Input};
use klingt::{AudioNode, CpalDevice, Handle, Klingt, ProcessContext};
use rtrb::{Consumer, Producer, RingBuffer};
use std::time::Instant;
use tracing::warn;

/// A source node that reads audio buffers from an rtrb ring buffer
pub struct RtrbSource {
    output_buffer: Consumer<Buffer>,
    /// Last sample value, used to avoid pops when buffer underruns
    last_sample: f32,
}

/// Message type for RtrbSource (no messages needed)
#[derive(Clone, Copy, Debug)]
pub enum RtrbSourceMessage {}

impl AudioNode for RtrbSource {
    type Message = RtrbSourceMessage;

    fn process(
        &mut self,
        _ctx: &ProcessContext,
        _messages: impl Iterator<Item = RtrbSourceMessage>,
        _inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // Fill output buffer by popping from the emulator consumer.
        // If no data available, hold last sample to avoid pops.
        if let Some(output) = outputs.first_mut() {
            match self.output_buffer.pop() {
                Ok(buf) => {
                    // Remember the last sample for continuity
                    if let Some(&last) = buf.last() {
                        self.last_sample = last;
                    }
                    *output = buf;
                }
                Err(_) => {
                    // No data available - fill with last sample to avoid pops
                    output.fill(self.last_sample);
                }
            }
        }
    }

    fn num_outputs(&self) -> usize {
        1
    }
}

pub struct GameTankAudio {
    klingt: Klingt,
    sample_rate: u32,
    #[allow(dead_code)]
    source_handle: Handle<RtrbSourceMessage>,
    producer: Producer<Buffer>,
    start_time: Instant,
    blocks_processed: u64,
}

impl GameTankAudio {
    /// Create the audio bridge. This creates an internal ring buffer (producer/consumer).
    /// The emulator run loop should pop from its own buffer and push into this `producer`
    /// via `push_buffer`.
    pub fn new() -> Self {
        let device = CpalDevice::default_output().expect("No audio device available");
        let sample_rate = device.sample_rate();
        let mut klingt = Klingt::new(sample_rate).with_output(device.create_sink());

        // create an internal ring buffer where the source reads from the consumer
        // and the app will push emulator buffers into the producer
        let (producer, consumer) = RingBuffer::<Buffer>::new(4096);

        // Create source node that will read from our internal consumer
        let source = RtrbSource {
            output_buffer: consumer,
            last_sample: 0.0,
        };

        let source_handle = klingt.add(source);
        klingt.output(&source_handle);

        Self {
            klingt,
            sample_rate,
            source_handle,
            producer,
            start_time: Instant::now(),
            blocks_processed: 0,
        }
    }

    /// Push a single emulator buffer into the internal ring buffer.
    /// Drops the buffer if the ring is full.
    pub fn push_buffer(&mut self, buf: Buffer) {
        if let Err(_b) = self.producer.push(buf) {
            warn!("audio bridge ring full; dropping audio buffer");
        }
    }

    /// Process audio blocks to keep up with real-time.
    /// Call this regularly from your main loop.
    pub fn process_audio(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rate = self.sample_rate as f64;
        // Each block is 64 samples. Calculate how many blocks should be processed by now.
        // Add a small buffer (4 blocks) to stay ahead.
        let target_blocks = (elapsed * rate / 64.0) as u64 + 4;
        
        while self.blocks_processed < target_blocks {
            self.klingt.process();
            self.blocks_processed += 1;
        }
    }
}
