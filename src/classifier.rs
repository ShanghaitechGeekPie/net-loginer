use anyhow::Result;
use image::imageops::FilterType;
use image::EncodableLayout;
use once_cell::sync::Lazy;
use onnxruntime::environment::Environment;
use onnxruntime::session::NdArray;
use onnxruntime::{ndarray::Array, session::Session};
use std::sync::Mutex;

static ENVIRONMENT: Lazy<Environment> = Lazy::new(|| {
    Environment::builder()
        .build()
        .expect("environment initialization exception!")
});

pub struct Classifier {
    session: Mutex<Session<'static>>,
    charset: Vec<String>,
    resize_param: [i64; 2],
    channels: usize,
}

impl Classifier {
    pub fn new<M: AsRef<[u8]>>(
        model: M,
        charset: Vec<String>,
        resize_param: [i64; 2],
        channels: usize,
    ) -> Result<Self> {
        let session = Mutex::new(
            ENVIRONMENT
                .new_session_builder()?
                .with_model_from_memory(model)?,
        );

        Ok(Self {
            session,
            charset,
            resize_param,
            channels,
        })
    }

    pub fn classification<I: AsRef<[u8]>>(&self, image: I) -> Result<String> {
        let image = {
            let image = image::load_from_memory(image.as_ref())?;
            let resize_width = if self.resize_param[0] == -1 {
                image.width() * self.resize_param[1] as u32 / image.height()
            } else {
                self.resize_param[0] as u32
            };
            image.resize(
                resize_width,
                self.resize_param[1] as u32,
                FilterType::Lanczos3,
            )
        };

        let image_bytes = if self.channels == 1 {
            EncodableLayout::as_bytes(image.to_luma8().as_ref()).to_vec()
        } else {
            image.to_rgb8().to_vec()
        };

        let width = image.width() as usize;
        let height = image.height() as usize;

        let image_vec = Array::from_shape_vec((self.channels, height, width), image_bytes)?;

        let tensor = Array::from_shape_fn(
            (1, self.channels as usize, height, width),
            |(_, c, i, j)| {
                let now = image_vec[[c as usize, i, j]] as f32;
                if self.channels == 1 {
                    ((now / 255f32) - 0.456f32) / 0.224f32
                } else {
                    match c {
                        0 => ((now / 255f32) - 0.485f32) / 0.229f32,
                        1 => ((now / 255f32) - 0.456f32) / 0.224f32,
                        2 => ((now / 255f32) - 0.406f32) / 0.225f32,
                        _ => unreachable!(),
                    }
                }
            },
        );

        let mut session = self.session.lock().unwrap();
        let result = session.run::<i64>(vec![&mut NdArray::new(tensor)])?;

        let mut last_item = 0;
        let classification = result[0]
            .iter()
            .filter_map(|&value| {
                if value != 0 && value != last_item {
                    last_item = value;
                    Some(self.charset[value as usize].to_string())
                } else {
                    None
                }
            })
            .collect::<String>();

        Ok(classification)
    }
}
