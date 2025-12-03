use dasp_graph::{Buffer, Input};
use klingt::{AudioNode, Handle, Klingt, ProcessContext};
use rtrb::{Consumer, Producer, RingBuffer};
use tracing::warn;

/// A source node that reads audio buffers from an rtrb ring buffer
pub struct RtrbSource {
    output_buffer: Consumer<Buffer>,
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
        // If no data available, output silence.
        if let Some(output) = outputs.first_mut() {
            match self.output_buffer.pop() {
                Ok(buf) => {
                    *output = buf;
                }
                Err(_) => {
                    // no data available from emulator -> silence
                    *output = Buffer::SILENT;
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
    #[allow(dead_code)]
    source_handle: Handle<RtrbSourceMessage>,
    producer: Producer<Buffer>,
}

impl GameTankAudio {
    /// Create the audio bridge. This creates an internal ring buffer (producer/consumer).
    /// The emulator run loop should pop from its own buffer and push into this `producer`
    /// via `push_buffer`.
    pub fn new() -> Self {
        let mut klingt = Klingt::default_output().expect("No audio device available");

        // create an internal ring buffer where the source reads from the consumer
        // and the app will push emulator buffers into the producer
        let (producer, consumer) = RingBuffer::<Buffer>::new(2048);

        // Create source node that will read from our internal consumer
        let source = RtrbSource {
            output_buffer: consumer,
        };

        let source_handle = klingt.add(source);
        klingt.output(&source_handle);

        Self {
            klingt,
            source_handle,
            producer,
        }
    }

    /// Push a single emulator buffer into the internal ring buffer.
    /// Drops the buffer if the ring is full.
    pub fn push_buffer(&mut self, buf: Buffer) {
        if let Err(_b) = self.producer.push(buf) {
            warn!("audio bridge ring full; dropping audio buffer");
        }
    }

    /// Process audio. Call regularly from your main loop.
    pub fn process_audio(&mut self) {
        self.klingt.process();
    }
}
