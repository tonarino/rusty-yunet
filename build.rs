fn main() {
    let mut build = cxx_build::bridge("src/lib.rs");
    let build = build
        .include("src/libfacedetection")
        .file("src/libfacedetection/facedetectcnn-data.cpp")
        .file("src/libfacedetection/facedetectcnn-model.cpp")
        .file("src/libfacedetection/facedetectcnn.cpp")
        .file("src/bridge_wrapper.cpp")
        .flag_if_supported("-std=c++11")
        .flag("-O3");

    // AVX (advanced vector extensions) support
    #[cfg(target_feature = "avx2")]
    let build = build.flag("-mavx2").define("_ENABLE_AVX2", None);

    // Fused multiply-add instruction support.
    #[cfg(target_feature = "fma")]
    let build = build.flag("-mfma");

    #[cfg(target_feature = "neon")]
    let build = build.define("_ENABLE_NEON", None);

    build.compile("rusty-yunet");

    println!("cargo:rerun-if-changed=src/libfacedetection/facedetectcnn-data.cpp");
    println!("cargo:rerun-if-changed=src/libfacedetection/facedetectcnn-model.cpp");
    println!("cargo:rerun-if-changed=src/libfacedetection/facedetectcnn.cpp");
    println!("cargo:rerun-if-changed=src/libfacedetection/facedetectcnn.h");
    println!("cargo:rerun-if-changed=src/bridge_wrapper.h");
    println!("cargo:rerun-if-changed=src/bridge_wrapper.cpp");
}
