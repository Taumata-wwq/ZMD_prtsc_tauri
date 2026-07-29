fn main() {
    // 1. Tauri 标准构建（图标、权限等）
    tauri_build::build();

    // 2. 任务1+5：编译 app.rc 嵌入 manifest（要求管理员权限）
    compile_manifest();
}

/// 编译 app.rc 嵌入 manifest
///
/// 根据工具链选择合适的资源编译器：
/// - GNU 工具链（x86_64-pc-windows-gnu）：使用 windres 编译为 .o 对象文件
/// - MSVC 工具链（x86_64-pc-windows-msvc）：使用 rc.exe 编译为 .res
///
/// manifest 中设置 requestedExecutionLevel=requireAdministrator，
/// 使应用启动时即触发 UAC 提权提示，与游戏权限级别一致。
fn compile_manifest() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let rc_path = std::path::Path::new(&manifest_dir).join("app.rc");

    if !rc_path.exists() {
        eprintln!("警告：app.rc 不存在，跳过 manifest 编译");
        return;
    }

    // 检测目标平台
    let target = std::env::var("TARGET").unwrap_or_default();
    let is_msvc = target.contains("msvc");

    if is_msvc {
        compile_with_rc_exe(&manifest_dir, &rc_path);
    } else {
        // GNU 工具链（含 windows-gnu）：使用 windres
        compile_with_windres(&manifest_dir, &rc_path);
    }

    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=app.manifest");
}

/// MSVC 工具链：使用 rc.exe 编译 .res
fn compile_with_rc_exe(manifest_dir: &str, rc_path: &std::path::Path) {
    let res_path = std::path::Path::new(manifest_dir).join("app.res");

    let rc_exe = find_rc_exe();
    match rc_exe {
        Some(rc) => {
            eprintln!("提示：使用 rc.exe: {}", rc.display());
            let output = std::process::Command::new(&rc)
                .arg(rc_path)
                .arg(format!("/fo:{}", res_path.to_string_lossy()))
                .output();

            match output {
                Ok(result) if result.status.success() => {
                    println!(
                        "cargo:rustc-link-arg-bins={}",
                        res_path.to_string_lossy()
                    );
                    eprintln!("提示：manifest 编译成功（rc.exe），已链接到 exe");
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    eprintln!("警告：rc.exe 编译 app.rc 失败");
                    eprintln!("  stderr: {}", stderr);
                    eprintln!("  stdout: {}", stdout);
                }
                Err(e) => {
                    eprintln!("警告：无法执行 rc.exe: {}", e);
                }
            }
        }
        None => {
            eprintln!("警告：未找到 rc.exe，跳过 manifest 编译（管理员提权将不生效）");
            eprintln!("警告：请安装 Windows SDK 或 Visual Studio Build Tools");
        }
    }
}

/// GNU 工具链：使用 windres 编译 .rc 为 .o
///
/// windres 是 MinGW 的资源编译器，可将 .rc 文件编译为 COFF 格式的对象文件，
/// 然后通过 GNU linker 链接到 exe 中。
fn compile_with_windres(manifest_dir: &str, rc_path: &std::path::Path) {
    let obj_path = std::path::Path::new(manifest_dir).join("app_manifest.o");

    // 优先使用 PATH 中的 windres
    let windres = find_windres();

    let windres = match windres {
        Some(w) => w,
        None => {
            eprintln!("警告：未找到 windres，跳过 manifest 编译（管理员提权将不生效）");
            eprintln!("警告：请安装 MinGW-w64 工具链");
            return;
        }
    };

    eprintln!("提示：使用 windres: {}", windres.display());

    // windres 命令：windres --input app.rc --output app_manifest.o --output-format=coff
    // 注意：windres 需要预处理 app.rc（#include <windows.h>），
    //       会自动调用 gcc 预处理器；若 gcc 不在 PATH，可用 --preprocessor 指定
    let output = std::process::Command::new(&windres)
        .arg("--input")
        .arg(rc_path)
        .arg("--output")
        .arg(&obj_path)
        .arg("--output-format=coff")
        .output();

    match output {
        Ok(result) if result.status.success() => {
            // 链接对象文件到 exe
            println!(
                "cargo:rustc-link-arg-bins={}",
                obj_path.to_string_lossy()
            );
            eprintln!("提示：manifest 编译成功（windres），已链接到 exe");
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            eprintln!("警告：windres 编译 app.rc 失败");
            eprintln!("  stderr: {}", stderr);
            eprintln!("  stdout: {}", stdout);
        }
        Err(e) => {
            eprintln!("警告：无法执行 windres: {}", e);
        }
    }
}

/// 查找 Windows SDK 的 rc.exe（仅 MSVC 工具链使用）
fn find_rc_exe() -> Option<std::path::PathBuf> {
    // 1. 直接尝试 PATH 中的 rc（使用 where.exe 而非 where，避免 PowerShell 别名冲突）
    if let Ok(output) = std::process::Command::new("where.exe").arg("rc.exe").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                let path = std::path::PathBuf::from(first_line);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    // 2. 查找 Windows Kit 安装目录
    let kits_root = std::env::var("WindowsSdkDir")
        .unwrap_or_else(|_| "C:\\Program Files (x86)\\Windows Kits\\10".to_string());
    let bin_root = std::path::Path::new(&kits_root).join("bin");

    if !bin_root.exists() {
        return None;
    }

    // 遍历 SDK 版本目录，取最新版本
    let mut versions: Vec<_> = std::fs::read_dir(&bin_root)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| e.file_name().to_string_lossy().starts_with("10."))
        .collect();

    versions.sort_by_key(|a| std::cmp::Reverse(a.file_name()));

    for v in versions {
        // 优先 x64，其次 x86
        for arch in &["x64", "x86"] {
            let rc = v.path().join(arch).join("rc.exe");
            if rc.exists() {
                return Some(rc);
            }
        }
    }

    None
}

/// 查找 windres（GNU 工具链使用）
fn find_windres() -> Option<std::path::PathBuf> {
    // 1. 直接尝试 PATH 中的 windres
    if let Ok(output) = std::process::Command::new("where.exe").arg("windres.exe").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                let path = std::path::PathBuf::from(first_line);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    // 2. 尝试常见 MinGW 安装路径
    let candidates = [
        r"C:\mingw64\bin\windres.exe",
        r"C:\msys64\mingw64\bin\windres.exe",
        r"C:\msys64\ucrt64\bin\windres.exe",
        r"C:\TDM-GCC-64\bin\windres.exe",
    ];

    for c in &candidates {
        let p = std::path::PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }

    None
}
