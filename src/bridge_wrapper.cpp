#include "bridge_wrapper.h"

rust::Vec<BridgeFace> wrapper_detect_faces(const unsigned char* rgbImageData, int width, int height, int step) {
    rust::Vec<BridgeFace> rust_faces;
    std::vector<FaceRect> faces = objectdetect_cnn(rgbImageData, width, height, step); 

    for (FaceRect f: faces) {
        BridgeFace bridge_face = BridgeFace {
            .score = f.score,
            .x = f.x,
            .y = f.y,
            .w = f.w,
            .h = f.h,
            .lm = {}
        };

        std::copy(std::begin(f.lm), std::end(f.lm), bridge_face.lm.begin());
        rust_faces.push_back(bridge_face);
    }

    return rust_faces;
}
