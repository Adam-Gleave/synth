use cpal::traits::{DeviceTrait, StreamTrait};
use dasp_graph::{Buffer, Input, Node};
use rtrb::{Producer, RingBuffer};

use std::marker::PhantomData;

pub struct CpalMonoSink<T: cpal::Sample> {
    marker: PhantomData<T>,
    buffer: Producer<f32>,
    stream: cpal::Stream,
}

impl<T: cpal::Sample> CpalMonoSink<T> {
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

impl<T: cpal::Sample> Node for CpalMonoSink<T> {
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
