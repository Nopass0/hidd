import { execSync } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as readline from "readline-sync";

function log(message: string, color: string = "reset") {
  const colors: { [key: string]: string } = {
    green: "\x1b[32m",
    yellow: "\x1b[33m",
    red: "\x1b[31m",
    cyan: "\x1b[36m",
    reset: "\x1b[0m",
  };
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function checkRustInstallation(): boolean {
  try {
    const rustVersion = execSync("rustc --version").toString().trim();
    log(`Rust уже установлен: ${rustVersion}`, "green");
    return true;
  } catch {
    log("Rust не установлен. Устанавливаем...", "yellow");
    try {
      execSync(
        "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
      );
      log("Rust успешно установлен", "green");
      return true;
    } catch (error) {
      log(`Ошибка при установке Rust: ${error}`, "red");
      return false;
    }
  }
}

function checkDependencies(): void {
  const visualStudioPath = path.join(
    process.env["ProgramFiles(x86)"] || "",
    "Microsoft Visual Studio",
    "Installer",
    "vswhere.exe"
  );

  if (!fs.existsSync(visualStudioPath)) {
    log("Необходимо установить Visual Studio Build Tools.", "yellow");
    log("Скачиваем Visual Studio Build Tools...");

    const buildToolsUrl = "https://aka.ms/vs/17/release/vs_buildtools.exe";
    execSync(`curl -o vs_buildtools.exe ${buildToolsUrl}`);
    execSync(
      `start /wait vs_buildtools.exe --quiet --wait --norestart --nocache --installPath "C:\\BuildTools" --add Microsoft.VisualStudio.Workload.VCTools`
    );
    fs.unlinkSync("vs_buildtools.exe");
  }
}

function getTargetUrl(): string {
  let url: string;
  do {
    url = readline.question("Введите URL для открытия в приложении: ");
    // Проверка правильности URL
    if (/^https?:\/\//i.test(url)) {
      return url;
    }
    log(
      "Пожалуйста, введите корректный URL (начинающийся с http:// или https://)",
      "yellow"
    );
  } while (true);
}

function buildProject(url: string): void {
  const mainRsPath = "src/main.rs";
  let mainRs = fs.readFileSync(mainRsPath, "utf-8");
  mainRs = mainRs.replace(/(with_url\(")([^"]+)("\))/g, `$1${url}$3`);
  fs.writeFileSync(mainRsPath, mainRs, { encoding: "utf-8" });

  log("Собираем релизную версию...", "yellow");
  execSync("cargo build --release");

  // Логирование содержимого директории
  console.log("Содержимое директории target/release:");
  fs.readdirSync("target/release").forEach((file) => console.log(file));

  const releaseExePath = "target/release/hidd.exe";

  // Проверка существования исполняемого файла
  if (fs.existsSync(releaseExePath)) {
    const releaseDir = "release_build";

    // Удаляем папку release_build, если она существует
    if (fs.existsSync(releaseDir)) {
      fs.rmSync(releaseDir, { recursive: true, force: true });
      log(`Старая папка ${releaseDir} была удалена.`, "yellow");
    }

    // Создаем новую папку release_build
    fs.mkdirSync(releaseDir, { recursive: true });
    log(`Создана новая папка ${releaseDir}.`, "green");

    // Копирование исполняемого файла
    try {
      fs.copyFileSync(releaseExePath, path.join(releaseDir, "hidd.exe"));
      log(
        `Сборка успешно завершена. Исполняемый файл находится в папке ${releaseDir}`,
        "green"
      );
    } catch (copyError: any) {
      log(`Ошибка при копировании файла: ${copyError.message}`, "red");
    }
  } else {
    log("Ошибка при сборке проекта: исполняемый файл не найден", "red");
  }
}

// Основной скрипт
log("Начинаем подготовку и сборку проекта...", "cyan");

if (!checkRustInstallation()) {
  process.exit(1);
}

checkDependencies();
const targetUrl = getTargetUrl();
buildProject(targetUrl);

log("Процесс завершен", "cyan");
