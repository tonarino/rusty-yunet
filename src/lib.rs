#![warn(clippy::all, clippy::clone_on_ref_ptr, clippy::mod_module_files)]

use image::{buffer::ConvertBuffer, open, Bgr, ImageBuffer};
use log::debug;
use serde::Serialize;
use std::{num::TryFromIntError, path::Path};
use thiserror::Error;
use tonari_math::Rect;

#[cxx::bridge]
mod ffi {
    // Shared type visible from both C++ and Rust
    #[derive(Debug)]
    struct BridgeFace {
        score: f32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        lm: [i32; 10],
    }

    unsafe extern "C++" {
        include!("rusty-yunet/src/bridge_wrapper.h");

        unsafe fn wrapper_detect_faces(
            rgb_image_data: *const u8,
            width: i32,
            height: i32,
            step: i32,
        ) -> Vec<BridgeFace>;
    }
}

#[derive(Error, Debug)]
pub enum YuNetError {
    #[error("Invalid input file")]
    InvalidFile,
    #[error("Image error")]
    ImageError(#[from] image::ImageError),
    #[error("Face detection failed")]
    FaceDetectionFailed,
}

/// NOTE: "right" and "left" are defined in the natural face sense;
/// a person's right eye is seen on the left side of the screen.
#[derive(Debug, Clone, Serialize)]
pub struct FaceLandmarks<T> {
    pub right_eye: (T, T),
    pub left_eye: (T, T),
    pub nose: (T, T),
    pub mouth_right: (T, T),
    pub mouth_left: (T, T),
}

impl FaceLandmarks<u16> {
    fn from_yunet_landmark_array(landmarks: &[i32; 10]) -> Result<Self, TryFromIntError> {
        Ok(Self {
            right_eye: (landmarks[0].try_into()?, landmarks[1].try_into()?),
            left_eye: (landmarks[2].try_into()?, landmarks[3].try_into()?),
            nose: (landmarks[4].try_into()?, landmarks[5].try_into()?),
            mouth_right: (landmarks[6].try_into()?, landmarks[7].try_into()?),
            mouth_left: (landmarks[8].try_into()?, landmarks[9].try_into()?),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Face {
    /// How confident (0..1) YuNet is that the rectangle represents a valid face.
    confidence: f32,
    /// Location of the face on absolute pixel coordinates.
    rectangle: Rect<u16>,
    /// The resolution of the image in which this face was detected (width, height).
    detection_dimensions: (u16, u16),
    /// Coordinates of five face landmarks.
    landmarks: FaceLandmarks<u16>,
}

impl Face {
    /// Conversion is fallible, as YuNet has been known to report faces with
    /// negative dimensions, rarely.
    fn from_yunet_bridge_face(
        face_rect: &ffi::BridgeFace,
        detection_dimensions: (u16, u16),
    ) -> Result<Self, TryFromIntError> {
        Ok(Self {
            confidence: face_rect.score,
            rectangle: Rect::with_size(
                face_rect.x.try_into()?,
                face_rect.y.try_into()?,
                face_rect.w.try_into()?,
                face_rect.h.try_into()?,
            ),
            landmarks: FaceLandmarks::from_yunet_landmark_array(&face_rect.lm)?,
            detection_dimensions,
        })
    }

    /// How confident (0..1) YuNet is that the rectangle is a face.
    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    /// Face rectangle in absolute pixel coordinates.
    pub fn rectangle(&self) -> Rect<u16> {
        self.rectangle
    }

    /// Face rectangle in normalized 0..1 coordinates.
    pub fn normalized_rectangle(&self) -> Rect<f32> {
        Rect::with_size(
            self.rectangle.left() as f32 / self.detection_dimensions.0 as f32,
            self.rectangle.top() as f32 / self.detection_dimensions.1 as f32,
            self.rectangle.width() as f32 / self.detection_dimensions.0 as f32,
            self.rectangle.height() as f32 / self.detection_dimensions.1 as f32,
        )
    }

    /// Coordinates of five face landmarks.
    pub fn landmarks(&self) -> &FaceLandmarks<u16> {
        &self.landmarks
    }

    /// Coordinates of five face landmarks in normalized 0..1 coordinates.
    pub fn normalized_landmarks(&self) -> FaceLandmarks<f32> {
        FaceLandmarks {
            right_eye: (
                self.landmarks.right_eye.0 as f32 / self.detection_dimensions.0 as f32,
                self.landmarks.right_eye.1 as f32 / self.detection_dimensions.1 as f32,
            ),
            left_eye: (
                self.landmarks.left_eye.0 as f32 / self.detection_dimensions.0 as f32,
                self.landmarks.left_eye.1 as f32 / self.detection_dimensions.1 as f32,
            ),
            nose: (
                self.landmarks.nose.0 as f32 / self.detection_dimensions.0 as f32,
                self.landmarks.nose.1 as f32 / self.detection_dimensions.1 as f32,
            ),
            mouth_right: (
                self.landmarks.mouth_right.0 as f32 / self.detection_dimensions.0 as f32,
                self.landmarks.mouth_right.1 as f32 / self.detection_dimensions.1 as f32,
            ),
            mouth_left: (
                self.landmarks.mouth_left.0 as f32 / self.detection_dimensions.0 as f32,
                self.landmarks.mouth_left.1 as f32 / self.detection_dimensions.1 as f32,
            ),
        }
    }
}

pub fn detect_faces<T: ConvertBuffer<ImageBuffer<Bgr<u8>, Vec<u8>>>>(
    image_buffer: &T,
) -> Result<Vec<Face>, YuNetError> {
    let image_buffer = image_buffer.convert();
    let (width, height) = (image_buffer.width() as u16, image_buffer.height() as u16);

    let faces = unsafe {
        crate::ffi::wrapper_detect_faces(
            image_buffer.as_ptr(),
            width as i32,
            height as i32,
            3 * width as i32,
        )
    };
    Ok(faces
        .into_iter()
        .filter_map(|f| match Face::from_yunet_bridge_face(&f, (width, height)) {
            Ok(face) => Some(face),
            Err(e) => {
                debug!("Yunet detected an invalid face: {f:?}: {e}. Discarding it.");
                None
            },
        })
        .collect())
}

pub fn detect_faces_from_file(filename: impl AsRef<Path>) -> Result<Vec<Face>, YuNetError> {
    let image_buffer = open(&filename)?.into_bgr8();
    detect_faces(&image_buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_sample_faces() {
        // Loads a sample with three faces clearly staggered in distance. Detecting the biggest
        // face with high confidence should be completely expected. Detecting the mid-sized face
        // is good, as it probably stretches what we consider "presence" in front of a normal
        // installation. Detecting the smallest face is very unrealistic and unnecessary.
        //
        // Detecting two faces with this test at this resolution can be considered a good result.
        assert_eq!(2, detect_faces_from_file("sample.jpg").unwrap().len());
    }
}
