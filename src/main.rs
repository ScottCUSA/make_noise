#![windows_subsystem = "windows"]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use fundsp::hacker::*;
use std::sync::mpsc::{self, Receiver};

use winsafe::prelude::*;
use winsafe::{gui, POINT, SIZE};

fn setup_ctrl_c() -> Receiver<()> {
    // create a channel to communicate ctrl+c was pressed
    let (tx_ctrl_c, rx_ctrl_c) = mpsc::channel();

    ctrlc::set_handler(move || {
        println!("\nCtrl+C pressed.");
        tx_ctrl_c.send(()).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    return rx_ctrl_c;
}

fn main() {
    // let rx_ctrl_c = setup_ctrl_c();
    let my = MyWindow::new(); // instantiate our main window
    if let Err(e) = my.wnd.run_main(None) {
        // ... and run it
        eprintln!("{}", e);
    }
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    let mut pink_noise = (pink() >> declick_s(1.0)) | (pink() >> declick_s(1.0));

    pink_noise.reset(Some(sample_rate));

    let mut next_value = move || pink_noise.get_stereo();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
    )?;
    stream.play()?;
    loop{}
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

#[derive(Clone)]
pub struct MyWindow {
    wnd: gui::WindowMain,   // responsible for managing the window
    btn_hello: gui::Button, // a button
}

impl MyWindow {
    pub fn new() -> Self {
        let wnd = gui::WindowMain::new(
            // instantiate the window manager
            gui::WindowMainOpts {
                title: "Make Noise".to_string(),
                size: SIZE::new(200, 200),
                ..Default::default() // leave all other options as default
            },
        );

        let btn_hello = gui::Button::new(
            &wnd, // the window manager is the parent of our button
            gui::ButtonOpts {
                text: "&Make Noise".to_string(),
                position: POINT::new(20, 20),
                ..Default::default()
            },
        );

        let new_self = Self { wnd, btn_hello };
        new_self.events(); // attach our events
        new_self
    }

    fn events(&self) {
        self.btn_hello.on().bn_clicked({
            let wnd = self.wnd.clone(); // clone so it can be passed into the closure
            move || {
                std::thread::spawn(|| {
                    let host = cpal::default_host();
                    let device = host
                        .default_output_device()
                        .expect("failed to find a default output device");
                    let config = device.default_output_config().unwrap();
                    match config.sample_format() {
                        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()).unwrap(),
                        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()).unwrap(),
                        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()).unwrap(),
                    }
                });
                Ok(())
            }
        });
    }
}
