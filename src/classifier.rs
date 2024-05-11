use anyhow::Result;
use once_cell::sync::Lazy;
use onnxruntime::environment::Environment;
use onnxruntime::session::NdArray;
use onnxruntime::{ndarray::Array, session::Session};
use resize::Pixel::RGB8;
use resize::Type::Lanczos3;
use rgb::FromSlice;
use std::sync::Mutex;
use zune_jpeg::JpegDecoder;

static ENVIRONMENT: Lazy<Environment> = Lazy::new(|| {
    Environment::builder()
        .build()
        .expect("environment initialization exception!")
});

#[derive(PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum ModelChannels {
    Gray = 1,
    RGB = 3,
}

pub enum ResizeParam {
    FixedWidth(usize),
    FixedHeight(usize),
    FixedSize(usize, usize),
}

impl ResizeParam {
    pub fn get_param(&self, image_info: (usize, usize)) -> (usize, usize) {
        let (origin_width, origin_height) = (image_info.0 as f32, image_info.1 as f32);

        match self {
            ResizeParam::FixedWidth(width) => {
                let height = (origin_height * *width as f32 / origin_width).round() as usize;
                (*width, height)
            }
            ResizeParam::FixedHeight(height) => {
                let width = (origin_width * *height as f32 / origin_height).round() as usize;
                (width, *height)
            }
            ResizeParam::FixedSize(width, height) => (*width, *height),
        }
    }
}

pub struct Classifier {
    session: Mutex<Session<'static>>,
    charset: Vec<String>,
    resize_param: ResizeParam,
    channels: ModelChannels,
}

impl Classifier {
    pub fn new<M: AsRef<[u8]>>(
        model: M,
        charset: Vec<String>,
        resize_param: ResizeParam,
        channels: ModelChannels,
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
        let (image, width, height) = self.resize_image(image)?;

        let image_bytes = match self.channels {
            ModelChannels::Gray => {
                let mut gray_image = vec![0; image.len() / 3];
                for (i, pixels) in image.chunks(3).enumerate() {
                    let gray = 0.2989 * pixels[0] as f32
                        + 0.5870 * pixels[1] as f32
                        + 0.1140 * pixels[2] as f32;
                    gray_image[i] = gray as u8;
                }
                gray_image
            }
            ModelChannels::RGB => image,
        };

        let tensor = Array::from_shape_fn(
            (1, self.channels as usize, height, width),
            |(_, c, i, j)| {
                let now = image_bytes[(i * width + j) * self.channels as usize + c] as f32;
                let (mean, std) = if self.channels == ModelChannels::Gray {
                    (0.456f32, 0.224f32)
                } else {
                    match c {
                        0 => (0.485f32, 0.229f32),
                        1 => (0.456f32, 0.224f32),
                        2 => (0.406f32, 0.225f32),
                        _ => unreachable!(),
                    }
                };
                ((now / 255f32) - mean) / std
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

    fn resize_image<I: AsRef<[u8]>>(&self, image: I) -> Result<(Vec<u8>, usize, usize)> {
        let mut decoder = JpegDecoder::new(image.as_ref());
        let image = decoder.decode()?;
        let image_info = decoder.info().unwrap();

        let (resize_width, resize_height) = self
            .resize_param
            .get_param((image_info.width as usize, image_info.height as usize));

        let mut resizer = resize::new(
            image_info.width as usize,
            image_info.height as usize,
            resize_width,
            resize_height,
            RGB8,
            Lanczos3,
        )?;

        let mut resized_image = vec![0; resize_width * resize_height as usize * 3];
        resizer.resize(&image.as_rgb(), resized_image.as_rgb_mut())?;

        Ok((resized_image, resize_width, resize_height as usize))
    }
}
