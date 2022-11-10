#![allow(unreachable_code)]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use fundsp::hacker::*;
use std::sync::mpsc::{self, Receiver};

fn main() {
    // create a channel to communicate ctrl+c was pressed
    let (tx_ctrl_c, rx_ctrl_c) = mpsc::channel();

    ctrlc::set_handler(move || {
        println!("\nCtrl+C pressed.");
        tx_ctrl_c.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), &rx_ctrl_c).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), &rx_ctrl_c).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), &rx_ctrl_c).unwrap(),
    }
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx_ctrl_c: &Receiver<()>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let mut c = pink() | pink();

    c.reset(Some(sample_rate));

    let mut next_value = move || c.get_stereo();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
    )?;
    stream.play()?;

    // wait for Ctrl+C
    _ = rx_ctrl_c.recv();

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left: T = cpal::Sample::from::<f32>(&(sample.0 as f32));
        let right: T = cpal::Sample::from::<f32>(&(sample.1 as f32));

        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = left;
            } else {
                *sample = right;
            }
        }
    }
}
