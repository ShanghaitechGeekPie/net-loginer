use image::imageops::FilterType;
use image::EncodableLayout;
use once_cell::sync::Lazy;
use onnxruntime::environment::Environment;
use onnxruntime::{ndarray::Array, session::Session};
use std::error::Error;
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
    ) -> Result<Self, Box<dyn Error>> {
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

    pub fn classification<I: AsRef<[u8]>>(&self, image: I) -> Result<String, Box<dyn Error>> {
        let image = {
            let origin_image = image::load_from_memory(image.as_ref())?;
            match self.resize_param[0] {
                -1 => origin_image.resize(
                    origin_image.width() * self.resize_param[1] as u32 / origin_image.height(),
                    self.resize_param[1] as u32,
                    FilterType::Lanczos3,
                ),
                _ => origin_image.resize(
                    self.resize_param[0] as u32,
                    self.resize_param[1] as u32,
                    FilterType::Lanczos3,
                ),
            }
        };

        let image_bytes = if self.channels == 1 {
            EncodableLayout::as_bytes(image.to_luma8().as_ref()).to_vec()
        } else {
            image.to_rgb8().to_vec()
        };

        let width = image.width() as usize;
        let height = image.height() as usize;

        let image = Array::from_shape_vec((self.channels, height, width), image_bytes)?;

        let mut tensor = Array::from_shape_vec(
            (1, self.channels, height, width),
            vec![0f32; height * width],
        )?;

        for i in 0..height {
            for j in 0..width {
                let now = image[[0, i, j]] as f32;
                if self.channels == 1 {
                    tensor[[0, 0, i, j]] = ((now / 255f32) - 0.456f32) / 0.224f32;
                } else {
                    let r = image[[0, i, j]] as f32;
                    let g = image[[1, i, j]] as f32;
                    let b = image[[2, i, j]] as f32;
                    tensor[[0, 0, i, j]] = ((r / 255f32) - 0.485f32) / 0.229f32;
                    tensor[[0, 1, i, j]] = ((g / 255f32) - 0.456f32) / 0.224f32;
                    tensor[[0, 2, i, j]] = ((b / 255f32) - 0.406f32) / 0.225f32;
                }
            }
        }

        let mut session = self.session.lock().unwrap();
        let result = session.run::<_, i64, _>(vec![tensor])?;

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
