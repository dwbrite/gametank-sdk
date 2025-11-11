use std::collections::VecDeque;
use std::ops::IndexMut;
use dasp_graph::{Buffer, Input, NodeData};
use klingt::{AudioNode, Klingt};
use rtrb::{Consumer, Producer, RingBuffer};
use tracing::{debug, info, trace, warn};
use petgraph::prelude::NodeIndex;
use klingt::nodes::sink::CpalMonoSink; // adjust path if different in your klingt version

#[derive(Debug)]
pub struct RtrbSource {
    output_buffer: Consumer<Buffer>,
}

pub struct GameTankAudio {
    klingt: Klingt<GTNode>,
    idx_in: NodeIndex,
    idx_out: NodeIndex,
    producer: Producer<Buffer>, // internal producer we push emulator data into
}

impl GameTankAudio {
    /// Create the audio bridge. This creates an internal ring buffer (producer/consumer).
    /// The emulator run loop should pop from its own buffer and push into this `producer`
    /// via `push_buffer`.
    pub fn new() -> Self {
        let mut klingt = Klingt::default();

        // create an internal ring buffer where the UI/bridge reads from the consumer
        // and the app will push emulator buffers into the producer
        let (producer, consumer) = RingBuffer::<Buffer>::new(2048);

        // Create sink node
        let sink = CpalMonoSink::default();
        let out_node = NodeData::new1(GTNode::CpalMonoSink(sink));

        // Create source node that will read from our internal consumer
        let gt_node = NodeData::new1(GTNode::GameTankSource(RtrbSource {
            output_buffer: consumer,
        }));

        let idx_in = klingt.add_node(gt_node);
        let idx_out = klingt.add_node(out_node);

        // route source -> sink
        klingt.add_edge(idx_in, idx_out, ());

        Self {
            klingt,
            idx_in,
            idx_out,
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

    /// Process audio until either sink can't accept a block or source has no data.
    /// Call regularly from your audio thread / main loop.
    pub fn process_audio(&mut self) {
        // debug/log the state so we can see why nothing flows
        let ready_to_output = if let GTNode::GameTankSource(src) = &mut self.klingt.index_mut(self.idx_in).node {
            src.output_buffer.slots()
        } else { 0 };
        let sink_slots = if let GTNode::CpalMonoSink(sink) = &mut self.klingt.index_mut(self.idx_out).node {
            sink.buffer.slots()
        } else { 0 };

        let mut ready_to_output = ready_to_output;
        let mut can_output = sink_slots >= 128 && ready_to_output >= 1;

        while can_output {
            self.klingt.processor.process(&mut self.klingt.graph, self.idx_out);

            if let GTNode::GameTankSource(src) = &mut self.klingt.index_mut(self.idx_in).node {
                ready_to_output = src.output_buffer.slots();
            }
            if let GTNode::CpalMonoSink(sink) = &mut self.klingt.index_mut(self.idx_out).node {
                can_output = sink.buffer.slots() >= 128 && ready_to_output >= 1;
            };
        }
    }
}

impl AudioNode for RtrbSource {
    fn process(&mut self, _inputs: &[Input], output: &mut [Buffer]) {
        // Fill each output buffer slot by popping from the emulator consumer.
        // If the emulator has fewer buffers than 'output.len()', fill remaining with SILENT.
        let mut i = 0usize;
        while i < output.len() {
            match self.output_buffer.pop() {
                Ok(buf) => {
                    output[i] = buf.clone();
                    i += 1;
                }
                Err(_) => {
                    // no more data available from emulator -> silence the rest
                    for out in output[i..].iter_mut() {
                        *out = Buffer::SILENT;
                    }
                    return;
                }
            }
        }
    }
}

#[enum_delegate::implement(AudioNode, pub trait AudioNode { fn process(&mut self, inputs: &[Input], output: &mut [Buffer]);})]
pub enum GTNode {
    CpalMonoSink(CpalMonoSink),
    GameTankSource(RtrbSource),
}
