use anyhow::Result;
use ndarray::Array;
use ort::session::Session;
use resize::Pixel::RGB8;
use resize::Type::Lanczos3;
use rgb::FromSlice;
use zune_jpeg::JpegDecoder;

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
    session: Session,
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
        let session = Session::builder()?.commit_from_memory(model.as_ref())?;

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
            ModelChannels::Gray => image
                .chunks(3)
                .map(|pixels| {
                    (0.2989 * pixels[0] as f32
                        + 0.5870 * pixels[1] as f32
                        + 0.1140 * pixels[2] as f32) as u8
                })
                .collect::<Vec<_>>(),
            ModelChannels::RGB => image,
        };

        let mean_std = match self.channels {
            ModelChannels::Gray => vec![(0.456f32, 0.224f32)],
            ModelChannels::RGB => vec![
                (0.485f32, 0.229f32),
                (0.456f32, 0.224f32),
                (0.406f32, 0.225f32),
            ],
        };

        let tensor = Array::from_shape_fn(
            (1, self.channels as usize, height, width),
            |(_, c, i, j)| {
                let now = image_bytes[(i * width + j) * self.channels as usize + c] as f32;
                let (mean, std) = mean_std[c];
                ((now / 255f32) - mean) / std
            },
        );

        let result = self.session.run(ort::inputs![tensor]?)?;
        let result = result[0].try_extract_tensor::<i64>()?;

        let mut last_item = 0;
        let classification = result
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

        let mut resized_image = vec![0; resize_width * resize_height * 3];
        resizer.resize(image.as_rgb(), resized_image.as_rgb_mut())?;

        Ok((resized_image, resize_width, resize_height))
    }
}
