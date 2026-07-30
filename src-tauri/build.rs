fn main() {
    tauri_build::build();
    compile_manifest();
}

// 编译 app.rc 嵌入 manifest（仅 release 构建）
//
// debug 跳过：dev 模式下 requireAdministrator manifest 会让 exe 启动即触发 UAC，
// 提权后的新进程不继承 Tauri CLI 注入的 dev server 环境变量，导致 run() 立即退出。
// release 根据 target 选择资源编译器：MSVC 用 rc.exe，GNU 用 windres。
fn compile_manifest() {
    let is_debug = std::env::var("DEBUG")
        .map(|v| v == "true")
        .unwrap_or(false);
    if is_debug {
        eprintln!("提示：debug 构建，跳过 manifest 编译");
        println!("cargo:rerun-if-changed=app.rc");
        println!("cargo:rerun-if-changed=app.manifest");
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let rc_path = std::path::Path::new(&manifest_dir).join("app.rc");

    if !rc_path.exists() {
        eprintln!("警告：app.rc 不存在，跳过 manifest 编译");
        return;
    }

    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("msvc") {
        compile_with_rc_exe(&manifest_dir, &rc_path);
    } else {
        compile_with_windres(&manifest_dir, &rc_path);
    }

    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=app.manifest");
}

// 执行资源编译命令并处理输出，成功时将产物链接到 exe
fn run_resource_compiler(
    tool_name: &str,
    mut command: std::process::Command,
    output_path: &std::path::Path,
) {
    match command.output() {
        Ok(result) if result.status.success() => {
            println!("cargo:rustc-link-arg-bins={}", output_path.to_string_lossy());
            eprintln!("提示：manifest 编译成功（{}），已链接到 exe", tool_name);
        }
        Ok(result) => {
            eprintln!("警告：{} 编译 app.rc 失败", tool_name);
            eprintln!("  stderr: {}", String::from_utf8_lossy(&result.stderr));
            eprintln!("  stdout: {}", String::from_utf8_lossy(&result.stdout));
        }
        Err(e) => {
            eprintln!("警告：无法执行 {}: {}", tool_name, e);
        }
    }
}

// MSVC 工具链：使用 rc.exe 编译 .res
fn compile_with_rc_exe(manifest_dir: &str, rc_path: &std::path::Path) {
    let Some(rc_exe) = find_rc_exe() else {
        eprintln!("警告：未找到 rc.exe，跳过 manifest 编译（管理员提权将不生效）");
        eprintln!("警告：请安装 Windows SDK 或 Visual Studio Build Tools");
        return;
    };
    eprintln!("提示：使用 rc.exe: {}", rc_exe.display());
    let res_path = std::path::Path::new(manifest_dir).join("app.res");
    let mut cmd = std::process::Command::new(&rc_exe);
    cmd.arg(rc_path).arg(format!("/fo:{}", res_path.to_string_lossy()));
    run_resource_compiler("rc.exe", cmd, &res_path);
}

// GNU 工具链：使用 windres 编译 .rc 为 .o
fn compile_with_windres(manifest_dir: &str, rc_path: &std::path::Path) {
    let Some(windres) = find_windres() else {
        eprintln!("警告：未找到 windres，跳过 manifest 编译（管理员提权将不生效）");
        eprintln!("警告：请安装 MinGW-w64 工具链");
        return;
    };
    eprintln!("提示：使用 windres: {}", windres.display());
    let obj_path = std::path::Path::new(manifest_dir).join("app_manifest.o");
    let mut cmd = std::process::Command::new(&windres);
    cmd.arg("--input").arg(rc_path)
        .arg("--output").arg(&obj_path)
        .arg("--output-format=coff");
    run_resource_compiler("windres", cmd, &obj_path);
}

// 通过 where.exe 查找 PATH 中的可执行文件（用 where.exe 避免 PowerShell 的 where 别名冲突）
fn find_in_path(exe_name: &str) -> Option<std::path::PathBuf> {
    let output = std::process::Command::new("where.exe").arg(exe_name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?;
    let path = std::path::PathBuf::from(first_line);
    if path.exists() { Some(path) } else { None }
}

// 查找 Windows SDK 的 rc.exe（仅 MSVC 工具链）
fn find_rc_exe() -> Option<std::path::PathBuf> {
    if let Some(p) = find_in_path("rc.exe") { return Some(p); }
    let kits_root = std::env::var("WindowsSdkDir")
        .unwrap_or_else(|_| "C:\\Program Files (x86)\\Windows Kits\\10".to_string());
    let bin_root = std::path::Path::new(&kits_root).join("bin");
    if !bin_root.exists() { return None; }
    // 遍历 SDK 版本目录，取最新版本
    let mut versions: Vec<_> = std::fs::read_dir(&bin_root).ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| e.file_name().to_string_lossy().starts_with("10."))
        .collect();
    versions.sort_by_key(|a| std::cmp::Reverse(a.file_name()));
    for v in versions {
        for arch in &["x64", "x86"] {
            let rc = v.path().join(arch).join("rc.exe");
            if rc.exists() { return Some(rc); }
        }
    }
    None
}

// 查找 windres（GNU 工具链）
fn find_windres() -> Option<std::path::PathBuf> {
    if let Some(p) = find_in_path("windres.exe") { return Some(p); }
    let candidates = [
        r"C:\mingw64\bin\windres.exe",
        r"C:\msys64\mingw64\bin\windres.exe",
        r"C:\msys64\ucrt64\bin\windres.exe",
        r"C:\TDM-GCC-64\bin\windres.exe",
    ];
    candidates.iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.exists())
}
