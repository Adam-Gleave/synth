use synth_module::{SynthModule, oscillator::DeriveOscillator, sequencer::StepSequencer};
use synth_node::{
    sink::CpalMonoSink,
    source::{Clock, Level},
};

use cpal::traits::{DeviceTrait, HostTrait};
use dasp_graph::{BoxedNode, NodeData, Processor};
use petgraph::Directed;

type Graph = petgraph::Graph<NodeData<BoxedNode>, (), Directed, u32>;

fn main() -> Result<(), anyhow::Error> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config()?;

    let mut g = Graph::new();
    let mut p = Processor::with_capacity(1024);

    let clock = Clock::new(160.0, config.sample_rate().0);
    let clock_idx = g.add_node(NodeData::boxed1(clock));

    let sequencer = StepSequencer::<4>::new([
        Level::new(0.0),
        Level::new(1.0),
        Level::new(0.25),
        Level::new(0.5),
    ])
    .build_graph(&mut g);
    g.add_edge(clock_idx, sequencer.clock_in().unwrap(), ());

    let oscillator = DeriveOscillator::new(130.0, config.sample_rate().0).build_graph(&mut g);
    g.add_edge(
        sequencer.v_oct_out().unwrap(),
        oscillator.v_oct_in().unwrap(),
        (),
    );

    let sink = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let sink = CpalMonoSink::<f32>::new(&device, &config.into());
            BoxedNode::new(sink)
        }
        cpal::SampleFormat::I16 => {
            let sink = CpalMonoSink::<i16>::new(&device, &config.into());
            BoxedNode::new(sink)
        }
        cpal::SampleFormat::U16 => {
            let sink = CpalMonoSink::<u16>::new(&device, &config.into());
            BoxedNode::new(sink)
        }
    };

    let sink_idx = g.add_node(NodeData::boxed1(sink));
    g.add_edge(oscillator.sine_out().unwrap(), sink_idx, ());

    loop {
        p.process(&mut g, sink_idx);
    }
}
