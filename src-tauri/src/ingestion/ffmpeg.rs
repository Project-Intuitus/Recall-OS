use crate::error::{RecallError, Result};
use crate::llm::VideoFrame;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// Pre-compiled regex patterns for ffmpeg output parsing
static DURATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Duration: (\d+):(\d+):(\d+\.?\d*)").unwrap()
});
#[allow(dead_code)] // Used in get_video_info
static RESOLUTION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d{2,4})x(\d{2,4})").unwrap()
});
#[allow(dead_code)] // Used in get_video_info
static FPS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d+(?:\.\d+)?)\s*fps").unwrap()
});

pub struct FFmpeg {
    binary_path: PathBuf,
}

impl FFmpeg {
    pub fn new() -> Result<Self> {
        // Look for ffmpeg in resources or PATH
        let binary_path = if cfg!(debug_assertions) {
            // Development: look in src-tauri/resources folder or PATH
            let resources_path = PathBuf::from("src-tauri/resources/ffmpeg.exe");
            if resources_path.exists() {
                tracing::debug!("Using ffmpeg from: {:?}", resources_path);
                resources_path
            } else {
                // Also try relative to current dir
                let alt_path = PathBuf::from("resources/ffmpeg.exe");
                if alt_path.exists() {
                    tracing::debug!("Using ffmpeg from: {:?}", alt_path);
                    alt_path
                } else {
                    // Fall back to PATH
                    tracing::warn!("ffmpeg not found in resources, falling back to PATH");
                    PathBuf::from("ffmpeg")
                }
            }
        } else {
            // Production: use bundled binary
            PathBuf::from("resources/ffmpeg.exe")
        };

        Ok(Self { binary_path })
    }

    pub async fn get_duration(&self, video_path: &Path) -> Result<f64> {
        let video_path_str = video_path.to_string_lossy();
        let output = Command::new(&self.binary_path)
            .args([
                "-i",
                &video_path_str,
                "-hide_banner",
                "-f",
                "null",
                "-",
            ])
            .output()
            .map_err(|e| RecallError::FFmpeg(format!("Failed to run ffmpeg: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse duration from output: "Duration: 00:05:30.12"
        if let Some(caps) = DURATION_REGEX.captures(&stderr) {
            let hours: f64 = caps[1].parse().unwrap_or(0.0);
            let minutes: f64 = caps[2].parse().unwrap_or(0.0);
            let seconds: f64 = caps[3].parse().unwrap_or(0.0);
            Ok(hours * 3600.0 + minutes * 60.0 + seconds)
        } else {
            Err(RecallError::FFmpeg("Could not parse video duration".to_string()))
        }
    }

    pub async fn extract_keyframes(&self, video_path: &Path, fps: f64) -> Result<Vec<VideoFrame>> {
        let temp_dir = TempDir::new()?;
        let output_pattern = temp_dir.path().join("frame_%05d.jpg");

        let fps_str = format!("{}", fps);
        let video_path_str = video_path.to_string_lossy();
        let output_pattern_str = output_pattern.to_string_lossy();

        let output = Command::new(&self.binary_path)
            .args([
                "-i",
                &*video_path_str,
                "-vf",
                &format!("fps={}", fps_str),
                "-q:v",
                "2", // High quality JPEG
                &*output_pattern_str,
            ])
            .output()
            .map_err(|e| RecallError::FFmpeg(format!("Failed to extract frames: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RecallError::FFmpeg(format!("Frame extraction failed: {}", stderr)));
        }

        // Read extracted frames
        let mut frames = Vec::new();
        let mut frame_num = 1;
        let interval = 1.0 / fps;

        loop {
            let frame_path = temp_dir.path().join(format!("frame_{:05}.jpg", frame_num));
            if !frame_path.exists() {
                break;
            }

            let image_data = std::fs::read(&frame_path)?;
            let timestamp = (frame_num - 1) as f64 * interval;

            frames.push(VideoFrame {
                timestamp,
                image_data,
            });

            frame_num += 1;
        }

        Ok(frames)
    }

    pub async fn extract_audio(&self, video_path: &Path) -> Result<PathBuf> {
        let output_path = std::env::temp_dir().join(format!(
            "recall_audio_{}.mp3",
            uuid::Uuid::new_v4()
        ));

        let video_path_str = video_path.to_string_lossy();
        let output_path_str = output_path.to_string_lossy();

        let output = Command::new(&self.binary_path)
            .args([
                "-i",
                &*video_path_str,
                "-vn", // No video
                "-acodec",
                "libmp3lame",
                "-ac",
                "1", // Mono
                "-ar",
                "16000", // 16kHz sample rate
                "-y", // Overwrite
                &*output_path_str,
            ])
            .output()
            .map_err(|e| RecallError::FFmpeg(format!("Failed to extract audio: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RecallError::FFmpeg(format!("Audio extraction failed: {}", stderr)));
        }

        Ok(output_path)
    }

    pub async fn convert_to_mono_mp3(&self, audio_path: &Path) -> Result<PathBuf> {
        let output_path = std::env::temp_dir().join(format!(
            "recall_mono_{}.mp3",
            uuid::Uuid::new_v4()
        ));

        let audio_path_str = audio_path.to_string_lossy();
        let output_path_str = output_path.to_string_lossy();

        let output = Command::new(&self.binary_path)
            .args([
                "-i",
                &*audio_path_str,
                "-acodec",
                "libmp3lame",
                "-ac",
                "1", // Mono
                "-ar",
                "16000", // 16kHz
                "-y",
                &*output_path_str,
            ])
            .output()
            .map_err(|e| RecallError::FFmpeg(format!("Failed to convert audio: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RecallError::FFmpeg(format!("Audio conversion failed: {}", stderr)));
        }

        Ok(output_path)
    }

    pub async fn get_video_info(&self, video_path: &Path) -> Result<VideoInfo> {
        let video_path_str = video_path.to_string_lossy();
        let output = Command::new(&self.binary_path)
            .args([
                "-i",
                &*video_path_str,
                "-hide_banner",
            ])
            .output()
            .map_err(|e| RecallError::FFmpeg(format!("Failed to probe video: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        let duration = if let Some(caps) = DURATION_REGEX.captures(&stderr) {
            let hours: f64 = caps[1].parse().unwrap_or(0.0);
            let minutes: f64 = caps[2].parse().unwrap_or(0.0);
            let seconds: f64 = caps[3].parse().unwrap_or(0.0);
            hours * 3600.0 + minutes * 60.0 + seconds
        } else {
            0.0
        };

        let resolution = if let Some(caps) = RESOLUTION_REGEX.captures(&stderr) {
            let width: u32 = caps[1].parse().unwrap_or(0);
            let height: u32 = caps[2].parse().unwrap_or(0);
            (width, height)
        } else {
            (0, 0)
        };

        let fps = if let Some(caps) = FPS_REGEX.captures(&stderr) {
            caps[1].parse().unwrap_or(0.0)
        } else {
            0.0
        };

        Ok(VideoInfo {
            duration,
            width: resolution.0,
            height: resolution.1,
            fps,
        })
    }
}

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}
