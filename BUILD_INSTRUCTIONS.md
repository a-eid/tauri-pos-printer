# Build Instructions for Windows

This project uses GitHub Actions to automatically build the Windows application. You don't need to install Rust or any build tools locally!

## 🚀 Quick Start - Building on GitHub

### Option 1: Automatic Build (Recommended)

Every time you push to the `main` or `master` branch, GitHub Actions will automatically build the Windows app.

1. **Push your code to GitHub:**
   ```bash
   git init
   git add .
   git commit -m "Initial commit"
   git branch -M main
   git remote add origin https://github.com/YOUR_USERNAME/tauri-pos-printer.git
   git push -u origin main
   ```

2. **Check the build:**
   - Go to your GitHub repository
   - Click on the "Actions" tab
   - You'll see the "Build Windows App" workflow running
   - Wait for it to complete (usually takes 5-10 minutes)

3. **Download the built app:**
   - Click on the completed workflow run
   - Scroll down to "Artifacts"
   - Download one of:
     - `windows-installer-nsis` (NSIS installer - recommended)
     - `windows-installer-msi` (MSI installer)
     - `windows-executable` (standalone .exe)

### Option 2: Manual Build Trigger

You can manually trigger a build anytime:

1. Go to your GitHub repository
2. Click on the "Actions" tab
3. Select "Build Windows App" from the left sidebar
4. Click "Run workflow" button on the right
5. Click the green "Run workflow" button
6. Wait for the build to complete
7. Download the artifacts as described above

### Option 3: Create a Release

To create an official release with version number:

1. **Create a tag and push it:**
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. **Or manually trigger a release:**
   - Go to the "Actions" tab
   - Select "Release" workflow
   - Click "Run workflow"
   - Enter your version (e.g., `v1.0.0`)
   - Click "Run workflow"

3. **The release will be created:**
   - Go to the "Releases" section of your repository
   - You'll find a draft release with the installers attached
   - Edit the release notes and publish it

## 📦 What Gets Built

The GitHub Actions workflow creates:

1. **NSIS Installer** (`.exe`) - Most popular, recommended for end users
2. **MSI Installer** (`.msi`) - Alternative Windows installer format
3. **Standalone Executable** (`.exe`) - Direct executable file

## 🔧 Local Development

For local development and testing (not building):

```bash
# Install dependencies
pnpm install

# Run in development mode (requires Rust locally)
pnpm run tauri dev
```

**Note:** To run `tauri dev` locally, you would need to install Rust. But for building the final Windows app, just use GitHub Actions!

## 📝 GitHub Actions Workflows

This project includes two workflows:

### 1. `build-windows.yml`
- **Triggers:** Push to main/master, pull requests, or manual
- **Purpose:** Build the app for testing
- **Output:** Artifacts (installers) available for download from the Actions tab

### 2. `release.yml`
- **Triggers:** Git tags (v*) or manual with version input
- **Purpose:** Create official releases
- **Output:** GitHub Release with installers attached

## 🛠️ First-Time Setup on GitHub

1. **Create a new repository on GitHub**
2. **Push this code:**
   ```bash
   git init
   git add .
   git commit -m "Initial commit with thermal printer app"
   git branch -M main
   git remote add origin https://github.com/YOUR_USERNAME/tauri-pos-printer.git
   git push -u origin main
   ```

3. **GitHub Actions will automatically start building!**

## ⚡ Tips

- The first build may take 10-15 minutes due to dependency downloads
- Subsequent builds are faster (5-7 minutes) thanks to caching
- You can have multiple builds running simultaneously
- All builds are free on public repositories (2000 minutes/month on private repos)

## 🎯 Recommended Workflow

1. **Develop locally** (optional, requires Rust)
2. **Push to GitHub** when ready
3. **Let GitHub Actions build** the Windows app automatically
4. **Download and test** the installer from Artifacts
5. **Create a release** when you want to distribute the app

## 📥 Installing on Windows

Once you download the installer:

1. Run the `.exe` installer (NSIS or MSI)
2. Follow the installation wizard
3. The app will be installed and ready to use
4. Connect your thermal printer and start printing!

---

**No Rust installation needed for building!** ✨
Just push to GitHub and let the cloud do the work.

