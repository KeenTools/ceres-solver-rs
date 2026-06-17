fn main() {
    println!("cargo:rerun-if-changed=src/lib.h");
    println!("cargo:rerun-if-changed=src/lib.cpp");
    println!("cargo:rerun-if-changed=src/lib.rs");

    if let Ok(deps) = std::env::var("KEENTECH_DEPS_DIR") {
        println!("cargo:rerun-if-env-changed=KEENTECH_DEPS_DIR");
        let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        let subdir = match (os.as_str(), arch.as_str()) {
            ("linux", _) => "CERES_LINUX_CXX11",
            ("macos", "aarch64") => "CERES_OSX_arm64",
            ("macos", "x86_64") => "CERES_OSX_x86_64",
            ("windows", _) => "CERES_WIN",
            _ => panic!("KEENTECH_DEPS_DIR: unsupported target {os}/{arch}"),
        };
        let inc = format!("{deps}/{subdir}/include");
        std::env::set_var("CERES_RS_LIB_DIR", format!("{deps}/{subdir}/lib"));
        std::env::set_var(
            "CERES_RS_INCLUDE_DIRS",
            format!("{inc},{deps}/EIGEN_INCLUDE,{inc}/ceres/internal/miniglog"),
        );
        std::env::set_var("CERES_RS_LIBS", "ceres");
        std::env::set_var(
            "CERES_RS_FLAGS",
            if os == "windows" {
                "/std:c++14"
            } else {
                "-std=c++14"
            },
        );
    }

    let mut cc_build = cxx_build::bridge("src/lib.rs");
    cc_build.file("src/lib.cpp");

    #[cfg(all(feature = "source", feature = "custom"))]
    compile_error!("Features 'source' and 'custom' are mutually exclusive.");

    #[cfg(feature = "custom")]
    {
        if [
            "CERES_RS_LIB_DIR",
            "CERES_RS_INCLUDE_DIRS",
            "CERES_RS_LIBS",
            "CERES_RS_FLAGS",
        ]
        .iter()
        .all(|k| std::env::var(k).is_err())
        {
            panic!(
                "ceres-solver-sys (custom feature): no ceres configuration found. \
                 Set KEENTECH_DEPS_DIR to the keentech-deps repo root, or set the \
                 CERES_RS_* env vars explicitly."
            );
        }

        if let Ok(flags) = std::env::var("CERES_RS_FLAGS") {
            for flag in flags.split(',') {
                cc_build.flag(flag);
            }
        }
        if let Ok(include_dirs) = std::env::var("CERES_RS_INCLUDE_DIRS") {
            cc_build.includes(include_dirs.split(','));
        }
        if let Ok(lib_dir) = std::env::var("CERES_RS_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", lib_dir);
        }
        if let Ok(libs) = std::env::var("CERES_RS_LIBS") {
            for lib in libs.split(',') {
                println!("cargo:rustc-link-lib=static={}", lib);
            }
        }
        if let Ok(defines) = std::env::var("CERES_RS_DEFINES") {
            for define in defines.split(',') {
                if let Some((key, value)) = define.split_once('=') {
                    cc_build.define(key, value);
                }
            }
        }
    }
    #[cfg(feature = "source")]
    {
        cc_build.flag("-std=c++17");

        cc_build.includes(std::env::split_paths(
            &std::env::var("DEP_CERES_INCLUDE").unwrap(),
        ));
        println!("cargo:rustc-link-lib=static=glog");
        println!("cargo:rustc-link-lib=static=ceres");
    }
    #[cfg(not(any(feature = "source", feature = "custom")))]
    {
        cc_build.flag("-std=c++17");

        if let Ok(library) = pkg_config::Config::new()
            .range_version("3.3.4".."4.0.0")
            .probe("eigen3")
        {
            library.include_paths.into_iter().for_each(|path| {
                cc_build.include(path);
            });
        }
        match pkg_config::Config::new()
            .range_version("2.0.0".."3.0.0")
            .probe("ceres")
        {
            Ok(library) => library.include_paths.into_iter().for_each(|path| {
                cc_build.include(path);
            }),
            Err(_) => println!("cargo:rustc-link-lib=ceres"),
        }
    }

    cc_build.compile("ceres-solver-sys");
}
