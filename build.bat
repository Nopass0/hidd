@echo off
SETLOCAL ENABLEDELAYEDEXPANSION

:: Функция для проверки установленного bun
:check_bun
where bun >nul 2>nul
IF ERRORLEVEL 1 (
    echo Bun не установлен. Устанавливаю...
    call :install_bun
) ELSE (
    echo Bun уже установлен.
)

:: Установка зависимостей
echo Установка зависимостей...
bun install

:: Запуск программы
echo Запуск build.ts через Bun...
bun run build.ts
exit /b

:: Функция для установки bun
:install_bun
:: Скачиваем и устанавливаем bun
powershell -Command "Invoke-WebRequest -Uri 'https://bun.sh/install' -OutFile 'bun-install.ps1'"
powershell -Command "Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass; .\bun-install.ps1"
del bun-install.ps1

:: Проверка успешности установки
where bun >nul 2>nul
IF ERRORLEVEL 1 (
    echo Не удалось установить bun. Пожалуйста, установите его вручную.
    exit /b 1
) ELSE (
    echo Bun успешно установлен.
)

exit /b
