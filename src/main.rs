use std::{
    marker::PhantomData,
    num::NonZeroUsize,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, SystemTime},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp_graph::{node::Pass, BoxedNode, Buffer, Input, Node, NodeData, Processor};
use petgraph::{graph::NodeIndex, Directed};
use rtrb::{Producer, RingBuffer};

type Graph = petgraph::Graph<NodeData<BoxedNode>, (), Directed, u32>;

struct Mul;

impl Node for Mul {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if inputs.len() == 2 && output.len() == 1 {
            let src = inputs[0].buffers();
            let scale = inputs[1].buffers();

            if src.len() == 1 && scale.len() == 1 {
                for i in 0..Buffer::LEN {
                    output[0][i] = src[0][i] * scale[0][i];
                }
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    }
}

struct Add;

impl Node for Add {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if inputs.len() == 2 && output.len() == 1 {
            let a = inputs[0].buffers();
            let b = inputs[1].buffers();

            if a.len() == 1 && b.len() == 1 {
                for i in 0..Buffer::LEN {
                    output[0][i] = a[0][i] + b[0][i];
                }
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    }
}

struct ModuleIO<T: Node + 'static> {
    inner: ModuleIO_Inner<T>,
}

enum ModuleIO_Inner<T: Node + 'static> {
    Disconnected(Option<T>),
    Connected(NodeIndex<u32>),
}

impl<T: Node + 'static> ModuleIO<T> {
    pub fn connected(index: NodeIndex<u32>) -> Self {
        Self {
            inner: ModuleIO_Inner::Connected(index),
        }
    }

    pub fn disconnected(node: T) -> Self {
        Self {
            inner: ModuleIO_Inner::Disconnected(Some(node)),
        }
    }

    pub fn connect(&mut self, graph: &mut Graph) {
        let inner = match &mut self.inner {
            ModuleIO_Inner::Disconnected(node) => {
                if let Some(node) = node.take() {
                    let idx = graph.add_node(NodeData::boxed1(node));
                    Some(ModuleIO_Inner::Connected(idx))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(inner) = inner {
            let _ = std::mem::replace(&mut self.inner, inner);
        }
    }

    pub fn index(&self) -> Option<NodeIndex<u32>> {
        match &self.inner {
            ModuleIO_Inner::Disconnected(_) => None,
            ModuleIO_Inner::Connected(idx) => Some(*idx),
        }
    }
}

struct MultiOscillator {
    v_oct: ModuleIO<PassOrDefault<Level>>,
    sine: ModuleIO<Sine>,
    square: ModuleIO<Square>,
    saw: ModuleIO<Saw>,
    triangle: ModuleIO<Triangle>,
}

impl MultiOscillator {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            v_oct: ModuleIO::disconnected(PassOrDefault::new(Level::new(0.0))),
            sine: ModuleIO::disconnected(Sine::new(freq, sample_rate)),
            square: ModuleIO::disconnected(Square::new(freq, sample_rate)),
            saw: ModuleIO::disconnected(Saw::new(freq, sample_rate)),
            triangle: ModuleIO::disconnected(Triangle::new(freq, sample_rate)),
        }
    }

    pub fn build_graph(mut self, graph: &mut Graph) -> Self {
        self.v_oct.connect(graph);
        self.sine.connect(graph);
        self.square.connect(graph);
        self.saw.connect(graph);
        self.triangle.connect(graph);

        graph.add_edge(self.v_oct.index().unwrap(), self.sine.index().unwrap(), ());
        graph.add_edge(
            self.v_oct.index().unwrap(),
            self.square.index().unwrap(),
            (),
        );
        graph.add_edge(self.v_oct.index().unwrap(), self.saw.index().unwrap(), ());
        graph.add_edge(
            self.v_oct.index().unwrap(),
            self.triangle.index().unwrap(),
            (),
        );

        self
    }

    pub fn v_oct_in(&self) -> Option<NodeIndex<u32>> {
        self.v_oct.index()
    }

    pub fn sine_out(&self) -> Option<NodeIndex<u32>> {
        self.sine.index()
    }

    pub fn square_out(&self) -> Option<NodeIndex<u32>> {
        self.square.index()
    }

    pub fn saw_out(&self) -> Option<NodeIndex<u32>> {
        self.saw.index()
    }

    pub fn triangle_out(&self) -> Option<NodeIndex<u32>> {
        self.triangle.index()
    }
}

struct PassOrDefault<T: Node> {
    pass: Pass,
    default: T,
}

impl<T: Node> PassOrDefault<T> {
    pub fn new(default: T) -> Self {
        Self {
            pass: Pass,
            default,
        }
    }
}

impl<T: Node> Node for PassOrDefault<T> {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if inputs.is_empty() {
            self.default.process(inputs, output);
        } else {
            self.pass.process(inputs, output);
        }
    }
}

enum LevelCommand {
    DeltaLevel(f32),
    SetLevel(f32),
}

struct Level {
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

struct Sine {
    freq: f32,
    phase: f32,
    sample_rate: f32,
}

impl Sine {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            freq,
            phase: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate as f32;
    }

    pub fn get_sample(&mut self, v_oct: Option<f32>) -> f32 {
        let freq = self.freq * 2_f32.powf(v_oct.unwrap_or_default());

        let t = 1.0 / self.sample_rate;
        self.phase = (self.phase + freq * t) % 1.0;

        (self.phase * 2.0 * std::f32::consts::PI).sin()
    }
}

impl Node for Sine {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match inputs.len() {
            0 => {
                for i in 0..Buffer::LEN {
                    for buffer in output.iter_mut() {
                        let sample = self.get_sample(None);
                        buffer[i] = sample;
                    }
                }
            }
            1 => {
                for i in 0..Buffer::LEN {
                    if inputs[0].buffers().len() != 1 {
                        panic!();
                    }

                    let v_oct_buf = &inputs[0].buffers()[0];

                    for buffer in output.iter_mut() {
                        let v_oct = v_oct_buf[i];
                        let sample = self.get_sample(Some(v_oct));
                        buffer[i] = sample;
                    }
                }
            }
            _ => panic!(),
        }
    }
}

struct Square {
    freq: f32,
    phase: f32,
    sample_rate: f32,
}

impl Square {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            freq,
            phase: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate as f32;
    }

    pub fn get_sample(&mut self, v_oct: Option<f32>) -> f32 {
        let freq = self.freq * 2_f32.powf(v_oct.unwrap_or_default());

        let t = 1.0 / self.sample_rate;
        self.phase = (self.phase + freq * t) % 1.0;

        if self.phase < 0.5 {
            1.0
        } else {
            -1.0
        }
    }
}

impl Node for Square {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match inputs.len() {
            0 => {
                for i in 0..Buffer::LEN {
                    for buffer in output.iter_mut() {
                        let sample = self.get_sample(None);
                        buffer[i] = sample;
                    }
                }
            }
            1 => {
                for i in 0..Buffer::LEN {
                    if inputs[0].buffers().len() != 1 {
                        panic!();
                    }

                    let v_oct_buf = &inputs[0].buffers()[0];

                    for buffer in output.iter_mut() {
                        let v_oct = v_oct_buf[i];
                        let sample = self.get_sample(Some(v_oct));
                        buffer[i] = sample;
                    }
                }
            }
            _ => panic!(),
        }
    }
}

struct Saw {
    freq: f32,
    phase: f32,
    sample_rate: f32,
}

impl Saw {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            freq,
            phase: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate as f32;
    }

    pub fn get_sample(&mut self, v_oct: Option<f32>) -> f32 {
        let freq = self.freq * 2_f32.powf(v_oct.unwrap_or_default());

        let t = 1.0 / self.sample_rate;
        self.phase = (self.phase + freq * t) % 1.0;

        (self.phase * 2.0) - 1.0
    }
}

impl Node for Saw {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match inputs.len() {
            0 => {
                for i in 0..Buffer::LEN {
                    for buffer in output.iter_mut() {
                        let sample = self.get_sample(None);
                        buffer[i] = sample;
                    }
                }
            }
            1 => {
                for i in 0..Buffer::LEN {
                    if inputs[0].buffers().len() != 1 {
                        panic!();
                    }

                    let v_oct_buf = &inputs[0].buffers()[0];

                    for buffer in output.iter_mut() {
                        let v_oct = v_oct_buf[i];
                        let sample = self.get_sample(Some(v_oct));
                        buffer[i] = sample;
                    }
                }
            }
            _ => panic!(),
        }
    }
}

struct Triangle {
    freq: f32,
    phase: f32,
    sample_rate: f32,
}

impl Triangle {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            freq,
            phase: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate as f32;
    }

    pub fn get_sample(&mut self, v_oct: Option<f32>) -> f32 {
        let freq = self.freq * 2_f32.powf(v_oct.unwrap_or_default());

        let t = 1.0 / self.sample_rate;
        self.phase = (self.phase + freq * t) % 1.0;

        if self.phase < 0.25 {
            self.phase * 4.0
        } else if self.phase < 0.75 {
            2.0 - (self.phase * 4.0)
        } else {
            self.phase * 4.0 - 4.0
        }
    }
}

impl Node for Triangle {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match inputs.len() {
            0 => {
                for i in 0..Buffer::LEN {
                    for buffer in output.iter_mut() {
                        let sample = self.get_sample(None);
                        buffer[i] = sample;
                    }
                }
            }
            1 => {
                for i in 0..Buffer::LEN {
                    if inputs[0].buffers().len() != 1 {
                        panic!();
                    }

                    let v_oct_buf = &inputs[0].buffers()[0];

                    for buffer in output.iter_mut() {
                        let v_oct = v_oct_buf[i];
                        let sample = self.get_sample(Some(v_oct));
                        buffer[i] = sample;
                    }
                }
            }
            _ => panic!(),
        }
    }
}

struct Clock {
    interval: u32,
    completed: u32,
}

impl Clock {
    pub const HIGH: f32 = 5.0;
    pub const LOW: f32 = 0.0;

    pub fn new(bpm: f32, sample_rate: u32) -> Self {
        let interval_s = 60.0 / bpm;
        let interval_samples = (interval_s * sample_rate as f32).ceil() as u32;

        Self {
            interval: interval_samples,
            completed: 0,
        }
    }
}

impl Node for Clock {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if !inputs.is_empty() {
            panic!();
        }

        for i in 0..Buffer::LEN {
            let sample = if self.completed == self.interval {
                Self::HIGH
            } else {
                Self::LOW
            };

            self.completed += 1;
            if self.completed > self.interval {
                self.completed = 0;
            }

            for buffer in output.iter_mut() {
                buffer[i] = sample;
            }
        }
    }
}

struct SequentialSwitch {
    cycled_inputs: usize,
    current_input: usize,
}

impl SequentialSwitch {
    const CLOCK_INDEX: usize = 0;
    const FIRST_INPUT_INDEX: usize = 1;

    pub fn new(cycled_inputs: usize) -> Self {
        Self {
            cycled_inputs,
            current_input: Self::FIRST_INPUT_INDEX,
        }
    }
}

impl Node for SequentialSwitch {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        for input in inputs {
            if input.buffers().len() > 1 {
                panic!();
            }
        }

        if self.cycled_inputs == 0 {
            for output in output.iter_mut() {
                *output = Buffer::SILENT;
            }

            return;
        }

        let clock_buf = inputs
            .get(Self::CLOCK_INDEX)
            .and_then(|input| input.buffers().get(0))
            .unwrap_or_else(|| &Buffer::SILENT);

        for i in 0..Buffer::LEN {
            let clock = clock_buf[i];

            if clock >= Clock::HIGH {
                self.current_input += 1;

                if self.current_input > self.cycled_inputs {
                    self.current_input = Self::FIRST_INPUT_INDEX;
                }
            }

            let sample = inputs
                .get(self.current_input)
                .and_then(|input| input.buffers().get(0))
                .and_then(|input_buf| input_buf.get(i))
                .unwrap_or_else(|| &0.0);

            for buffer in output.iter_mut() {
                buffer[i] = *sample;
            }
        }
    }
}

struct StepSequencer<const N: usize> {
    clock_in: ModuleIO<Pass>,
    levels: [ModuleIO<PassOrDefault<Level>>; N],
    level_switch: ModuleIO<SequentialSwitch>,
    v_oct_out: ModuleIO<Pass>,
}

impl<const N: usize> StepSequencer<N> {
    pub fn new(levels: [Level; N]) -> Self {
        let clock_in = ModuleIO::disconnected(Pass);
        let level_switch = ModuleIO::disconnected(SequentialSwitch::new(N));
        let levels = levels.map(|level| ModuleIO::disconnected(PassOrDefault::new(level)));
        let v_oct_out = ModuleIO::disconnected(Pass);

        Self {
            clock_in,
            level_switch,
            levels,
            v_oct_out,
        }
    }

    pub fn build_graph(mut self, graph: &mut Graph) -> Self {
        self.clock_in.connect(graph);
        self.level_switch.connect(graph);

        for level in self.levels.iter_mut() {
            level.connect(graph);
        }

        self.v_oct_out.connect(graph);

        for level in self.levels.iter().rev() {
            graph.add_edge(
                level.index().unwrap(),
                self.level_switch.index().unwrap(),
                (),
            );
        }

        graph.add_edge(self.clock_in.index().unwrap(), self.level_switch.index().unwrap(), ());
        graph.add_edge(self.level_switch.index().unwrap(), self.v_oct_out.index().unwrap(), ());

        self
    }

    pub fn clock_in(&self) -> Option<NodeIndex<u32>> {
        self.clock_in.index()
    }

    pub fn v_oct_in(&self, index: usize) -> Option<NodeIndex<u32>> {
        self.levels.get(index).and_then(|level| level.index())
    }

    pub fn v_oct_out(&self) -> Option<NodeIndex<u32>> {
        self.v_oct_out.index()
    }
}

struct Sink<T>
where
    T: cpal::Sample,
{
    marker: PhantomData<T>,
    buffer: Producer<f32>,
    stream: cpal::Stream,
}

impl<T> Sink<T>
where
    T: cpal::Sample,
{
    pub fn new(device: &cpal::Device, config: &cpal::StreamConfig) -> Self {
        let channels = config.channels as usize;

        let (producer, mut consumer) = RingBuffer::<f32>::new(4096);

        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    data.chunks_mut(channels).for_each(|frame| {
                        let value = T::from(&consumer.pop().unwrap_or_default());

                        frame.iter_mut().for_each(|sample| {
                            *sample = value;
                        });
                    });
                },
                |e| eprintln!("an error occured: {}", e),
            )
            .unwrap();

        Self {
            marker: PhantomData,
            buffer: producer,
            stream,
        }
    }
}

impl<T> Node for Sink<T>
where
    T: cpal::Sample,
{
    fn process(&mut self, inputs: &[Input], _output: &mut [Buffer]) {
        for input in inputs {
            let buffers = input.buffers();

            for i in 0..Buffer::LEN {
                for buffer in buffers {
                    while self.buffer.is_full() {}
                    self.buffer.push(buffer[i]).unwrap();
                }
            }
        }

        self.stream.play().unwrap();
    }
}

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

    let oscillator = MultiOscillator::new(130.0, config.sample_rate().0).build_graph(&mut g);
    g.add_edge(sequencer.v_oct_out().unwrap(), oscillator.v_oct_in().unwrap(), ());

    let sink = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let sink = Sink::<f32>::new(&device, &config.into());
            BoxedNode::new(sink)
        }
        cpal::SampleFormat::I16 => {
            let sink = Sink::<i16>::new(&device, &config.into());
            BoxedNode::new(sink)
        }
        cpal::SampleFormat::U16 => {
            let sink = Sink::<u16>::new(&device, &config.into());
            BoxedNode::new(sink)
        }
    };

    let sink_idx = g.add_node(NodeData::boxed1(sink));
    g.add_edge(oscillator.sine_out().unwrap(), sink_idx, ());

    loop {
        p.process(&mut g, sink_idx);
    }
}
