
@echo off
setlocal EnableExtensions DisableDelayedExpansion
title GravityAnime Release Manager
cd /d "%~dp0"

echo ==========================================
echo       GravityAnime Release Manager
echo ==========================================
echo.

where git >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Git is not installed or not in PATH.
    pause
    exit /b 1
)

git rev-parse --is-inside-work-tree >nul 2>&1
if errorlevel 1 (
    echo [ERROR] This folder is not a Git repository.
    echo Clone your GravityAnime repository first.
    pause
    exit /b 1
)

git remote get-url origin >nul 2>&1
if errorlevel 1 (
    echo [ERROR] No origin remote configured.
    pause
    exit /b 1
)

echo Current branch:
git branch --show-current
echo.

set /p "VERSION=Enter release version (e.g. 0.2.0): "
if not defined VERSION (
    echo [ERROR] Version cannot be empty.
    pause
    exit /b 1
)

set "TAG=%VERSION%"
if /i not "%TAG:~0,1%"=="v" set "TAG=v%TAG%"

echo.
echo Checking tag %TAG%...

git check-ref-format "refs/tags/%TAG%" >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Invalid Git tag name.
    pause
    exit /b 1
)

git rev-parse -q --verify "refs/tags/%TAG%" >nul 2>&1
if not errorlevel 1 (
    echo [ERROR] Tag already exists locally.
    pause
    exit /b 1
)

git ls-remote --exit-code --tags origin "refs/tags/%TAG%" >nul 2>&1
if not errorlevel 1 (
    echo [ERROR] Tag already exists on GitHub.
    pause
    exit /b 1
)

echo.
echo ==========================================
echo Release: %TAG%
echo ==========================================
echo.
git status --short
echo.

choice /c YN /m "Commit and publish this release"
if errorlevel 2 exit /b 0

echo.
echo [1/5] Staging files...
git add -A
if errorlevel 1 goto failed

echo.
echo [2/5] Creating commit if needed...
git diff --cached --quiet
if errorlevel 1 (
    git commit -m "Release %TAG%"
    if errorlevel 1 goto failed
) else (
    echo No changes to commit. Using current HEAD.
)

echo.
echo [3/5] Pushing branch...
git push -u origin HEAD
if errorlevel 1 goto failed

echo.
echo [4/5] Creating release tag...
git tag -a "%TAG%" -m "GravityAnime %TAG%"
if errorlevel 1 goto failed

echo.
echo [5/5] Pushing release tag...
git push origin "%TAG%"
if errorlevel 1 goto failed

echo.
echo ==========================================
echo        RELEASE TRIGGERED!
echo ==========================================
echo.
echo Version: %TAG%
echo.
echo GitHub Actions:
echo https://github.com/acolla9/GravityAnime/actions
echo.
echo GitHub Releases:
echo https://github.com/acolla9/GravityAnime/releases
echo.
echo GitHub Actions will build the NSIS installer
echo and publish the release automatically.
echo.
pause
exit /b 0

:failed
echo.
echo [ERROR] Release process failed.
echo Check the Git output above.
echo.
pause
exit /b 1
