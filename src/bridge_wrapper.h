#pragma once

#include "rusty-yunet/src/libfacedetection/facedetectcnn.h"
#include "rusty-yunet/src/lib.rs.h"
#include "rust/cxx.h"

#include <vector>

rust::Vec<BridgeFace> wrapper_detect_faces(const unsigned char* rgbImageData, int width, int height, int step);
