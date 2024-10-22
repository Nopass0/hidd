import { execSync } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as readline from "readline-sync";
import * as os from "os";

interface BuildConfig {
  platform: string;
  arch: string;
  target: string;
  extension: string;
  executableName: string;
}

const COLORS = {
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  red: "\x1b[31m",
  cyan: "\x1b[36m",
  reset: "\x1b[0m",
} as const;

function log(message: string, color: keyof typeof COLORS = "reset"): void {
  console.log(`${COLORS[color]}${message}${COLORS.reset}`);
}

function checkRustInstallation(): boolean {
  try {
    const rustVersion = execSync("rustc --version").toString().trim();
    log(`Rust установлен: ${rustVersion}`, "green");
    return true;
  } catch {
    log("Rust не установлен. Устанавливаем...", "yellow");
    try {
      if (process.platform === "win32") {
        execSync(
          "curl --proto '=https' --tlsv1.2 -sSf https://win.rustup.rs -o rustup-init.exe"
        );
        execSync("rustup-init.exe -y");
        fs.unlinkSync("rustup-init.exe");
      } else {
        execSync(
          "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
        );
      }
      log("Rust успешно установлен", "green");
      return true;
    } catch (error) {
      log(`Ошибка при установке Rust: ${error}`, "red");
      return false;
    }
  }
}

function checkCrossDependencies(): void {
  try {
    execSync("cross --version");
  } catch {
    log("Устанавливаем cross...", "yellow");
    execSync("cargo install cross");
  }
}

function checkPlatformDependencies(): void {
  if (process.platform === "win32") {
    const visualStudioPath = path.join(
      process.env["ProgramFiles(x86)"] || "",
      "Microsoft Visual Studio",
      "Installer",
      "vswhere.exe"
    );

    if (!fs.existsSync(visualStudioPath)) {
      log("Устанавливаем Visual Studio Build Tools...", "yellow");
      const buildToolsUrl = "https://aka.ms/vs/17/release/vs_buildtools.exe";
      execSync(`curl -o vs_buildtools.exe ${buildToolsUrl}`);
      execSync(
        `start /wait vs_buildtools.exe --quiet --wait --norestart --nocache ` +
          `--installPath "C:\\BuildTools" --add Microsoft.VisualStudio.Workload.VCTools`
      );
      fs.unlinkSync("vs_buildtools.exe");
    }
  } else if (process.platform === "darwin") {
    try {
      execSync("xcode-select -p");
    } catch {
      log("Устанавливаем Xcode Command Line Tools...", "yellow");
      execSync("xcode-select --install");
    }
  }
}

function getTargetUrl(): string {
  let url: string;
  do {
    url = readline.question("Введите URL для открытия в приложении: ");
    if (/^https?:\/\//i.test(url)) {
      return url;
    }
    log(
      "Пожалуйста, введите корректный URL (начинающийся с http:// или https://)",
      "yellow"
    );
  } while (true);
}

function updateSourceUrl(url: string): void {
  const mainRsPath = "src/main.rs";
  let mainRs = fs.readFileSync(mainRsPath, "utf-8");
  mainRs = mainRs.replace(/(with_url\(")([^"]+)("\))/g, `$1${url}$3`);
  fs.writeFileSync(mainRsPath, mainRs, { encoding: "utf-8" });
}

function getBuildConfigs(): BuildConfig[] {
  return [
    {
      platform: "windows",
      arch: "x64",
      target: "x86_64-pc-windows-msvc",
      extension: ".exe",
      executableName: "hidd.exe",
    },
    {
      platform: "darwin",
      arch: "x64",
      target: "x86_64-apple-darwin",
      extension: "",
      executableName: "hidd",
    },
    {
      platform: "darwin",
      arch: "arm64",
      target: "aarch64-apple-darwin",
      extension: "",
      executableName: "hidd",
    },
  ];
}

async function buildForPlatform(
  config: BuildConfig,
  url: string
): Promise<void> {
  log(`Сборка для ${config.platform} (${config.arch})...`, "yellow");

  const releaseDir = path.join(
    "release_build",
    `${config.platform}-${config.arch}`
  );
  fs.mkdirSync(releaseDir, { recursive: true });

  try {
    // Use native cargo for Windows builds when on Windows
    if (process.platform === "win32" && config.platform === "windows") {
      execSync(`cargo build --target ${config.target} --release`);
    } else {
      // Use cross for cross-compilation
      execSync(`cross build --target ${config.target} --release`);
    }

    const sourceFile = path.join(
      "target",
      config.target,
      "release",
      config.executableName
    );

    const targetFile = path.join(releaseDir, config.executableName);

    if (fs.existsSync(sourceFile)) {
      fs.copyFileSync(sourceFile, targetFile);

      // Установка прав на выполнение для Unix-подобных систем
      if (config.platform !== "windows") {
        fs.chmodSync(targetFile, 0o755);
      }

      // Создание .app бундла для macOS
      if (config.platform === "darwin") {
        createMacOSBundle(releaseDir, config);
      }

      log(
        `Сборка для ${config.platform} (${config.arch}) успешно завершена`,
        "green"
      );
    } else {
      throw new Error(`Исполняемый файл не найден: ${sourceFile}`);
    }
  } catch (error) {
    log(
      `Ошибка при сборке для ${config.platform} (${config.arch}): ${error}`,
      "red"
    );
  }
}

function createMacOSBundle(releaseDir: string, config: BuildConfig): void {
  const appDir = path.join(releaseDir, "Hidd.app");
  const contentsDir = path.join(appDir, "Contents");
  const macOSDir = path.join(contentsDir, "MacOS");
  const resourcesDir = path.join(contentsDir, "Resources");

  // Создаем структуру директорий
  fs.mkdirSync(macOSDir, { recursive: true });
  fs.mkdirSync(resourcesDir, { recursive: true });

  // Копируем исполняемый файл
  fs.copyFileSync(
    path.join(releaseDir, config.executableName),
    path.join(macOSDir, config.executableName)
  );

  // Создаем Info.plist
  const infoPlist = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Hidd</string>
    <key>CFBundleDisplayName</key>
    <string>Hidd</string>
    <key>CFBundleIdentifier</key>
    <string>com.yourcompany.hidd</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleExecutable</key>
    <string>${config.executableName}</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.11</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>`;

  fs.writeFileSync(path.join(contentsDir, "Info.plist"), infoPlist);
}

async function main() {
  log("Начинаем подготовку и сборку проекта...", "cyan");

  if (!checkRustInstallation()) {
    process.exit(1);
  }

  checkPlatformDependencies();
  checkCrossDependencies();

  const url = getTargetUrl();
  updateSourceUrl(url);

  // Очистка директории сборки
  if (fs.existsSync("release_build")) {
    fs.rmSync("release_build", { recursive: true, force: true });
  }

  // Получаем конфигурации для сборки
  const configs = getBuildConfigs();

  // Собираем для всех платформ
  for (const config of configs) {
    await buildForPlatform(config, url);
  }

  // Создание универсального бинарника для macOS, если собираем на macOS
  if (process.platform === "darwin") {
    const macosX64Dir = path.join("release_build", "darwin-x64", "hidd");
    const macosArmDir = path.join("release_build", "darwin-arm64", "hidd");
    const universalBinDir = path.join("release_build", "darwin-universal");

    if (fs.existsSync(macosX64Dir) && fs.existsSync(macosArmDir)) {
      fs.mkdirSync(universalBinDir, { recursive: true });
      execSync(
        `lipo -create -output "${path.join(
          universalBinDir,
          "hidd"
        )}" "${macosX64Dir}" "${macosArmDir}"`
      );
      log("Создан универсальный бинарник для macOS", "green");
    }
  }

  log("Процесс сборки завершен", "cyan");
}

main().catch((error) => {
  log(`Произошла ошибка: ${error}`, "red");
  process.exit(1);
});
