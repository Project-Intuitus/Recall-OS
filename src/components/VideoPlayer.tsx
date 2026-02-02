import { useRef, useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Play, Pause, Volume2, VolumeX, Maximize, SkipBack, SkipForward } from "lucide-react";
import clsx from "clsx";

interface VideoPlayerProps {
  filePath: string;
  currentTime?: number;
  onTimeUpdate?: (time: number) => void;
}

export default function VideoPlayer({ filePath, currentTime, onTimeUpdate }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const progressRef = useRef<HTMLDivElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [duration, setDuration] = useState(0);
  const [currentPosition, setCurrentPosition] = useState(0);
  const [playbackRate, setPlaybackRate] = useState(1);
  const [isLoaded, setIsLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Convert local file path to Tauri asset URL
  const videoUrl = convertFileSrc(filePath);

  // Seek to timestamp when currentTime prop changes
  useEffect(() => {
    if (currentTime !== undefined && videoRef.current && isLoaded) {
      videoRef.current.currentTime = currentTime;
      setCurrentPosition(currentTime);
    }
  }, [currentTime, isLoaded]);

  // Handle video metadata loaded
  const handleLoadedMetadata = () => {
    if (videoRef.current) {
      setDuration(videoRef.current.duration);
      setIsLoaded(true);
      setError(null);
    }
  };

  // Handle time update
  const handleTimeUpdate = () => {
    if (videoRef.current) {
      const time = videoRef.current.currentTime;
      setCurrentPosition(time);
      onTimeUpdate?.(time);
    }
  };

  // Handle play/pause
  const togglePlay = () => {
    if (videoRef.current) {
      if (isPlaying) {
        videoRef.current.pause();
        setIsPlaying(false);
      } else {
        videoRef.current.play()
          .then(() => setIsPlaying(true))
          .catch((err) => {
            // Handle autoplay restrictions or other play errors
            console.warn("Video play failed:", err);
            setError("Failed to play video. This may be due to browser autoplay restrictions.");
          });
      }
    }
  };

  // Handle mute
  const toggleMute = () => {
    if (videoRef.current) {
      videoRef.current.muted = !isMuted;
      setIsMuted(!isMuted);
    }
  };

  // Handle seek
  const handleSeek = (e: React.MouseEvent<HTMLDivElement>) => {
    if (progressRef.current && videoRef.current) {
      const rect = progressRef.current.getBoundingClientRect();
      // Guard against division by zero
      if (rect.width === 0 || duration === 0) return;
      const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const newTime = percent * duration;
      if (!isFinite(newTime)) return;
      videoRef.current.currentTime = newTime;
      setCurrentPosition(newTime);
    }
  };

  // Handle playback rate
  const changePlaybackRate = () => {
    const rates = [0.5, 1, 1.5, 2];
    const currentIndex = rates.indexOf(playbackRate);
    const nextRate = rates[(currentIndex + 1) % rates.length];
    if (videoRef.current) {
      videoRef.current.playbackRate = nextRate;
    }
    setPlaybackRate(nextRate);
  };

  // Skip forward/backward
  const skip = (seconds: number) => {
    if (videoRef.current) {
      videoRef.current.currentTime = Math.max(0, Math.min(duration, videoRef.current.currentTime + seconds));
    }
  };

  // Handle fullscreen
  const toggleFullscreen = async () => {
    if (videoRef.current) {
      try {
        if (document.fullscreenElement) {
          await document.exitFullscreen();
        } else {
          await videoRef.current.requestFullscreen();
        }
      } catch (err) {
        // Fullscreen may be blocked by browser or user preferences
        console.warn("Fullscreen request failed:", err);
      }
    }
  };

  // Handle error
  const handleError = () => {
    setError("Failed to load video. The file may be corrupted or unsupported.");
  };

  // Format time
  const formatTime = (seconds: number) => {
    if (!isFinite(seconds) || seconds < 0) return "0:00";
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  const progressPercent = duration > 0 && isFinite(currentPosition / duration)
    ? (currentPosition / duration) * 100
    : 0;

  if (error) {
    return (
      <div className="flex items-center justify-center h-full bg-slate-900 text-red-400 p-4">
        <p>{error}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-black">
      {/* Video element */}
      <div className="flex-1 relative flex items-center justify-center bg-black">
        <video
          ref={videoRef}
          src={videoUrl}
          className="max-w-full max-h-full"
          onLoadedMetadata={handleLoadedMetadata}
          onTimeUpdate={handleTimeUpdate}
          onPlay={() => setIsPlaying(true)}
          onPause={() => setIsPlaying(false)}
          onError={handleError}
          onClick={togglePlay}
        />

        {/* Loading overlay */}
        {!isLoaded && !error && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-900/80">
            <div className="animate-spin w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full" />
          </div>
        )}
      </div>

      {/* Controls */}
      <div className="bg-slate-800/95 backdrop-blur border-t border-slate-700 p-3">
        {/* Progress bar */}
        <div
          ref={progressRef}
          className="h-1.5 bg-slate-600 rounded-full cursor-pointer mb-3 group"
          onClick={handleSeek}
        >
          <div
            className="h-full bg-blue-500 rounded-full relative"
            style={{ width: `${progressPercent}%` }}
          >
            <div className="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
          </div>
        </div>

        {/* Control buttons */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {/* Skip back */}
            <button
              onClick={() => skip(-10)}
              className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
              title="Skip back 10s"
            >
              <SkipBack className="w-4 h-4" />
            </button>

            {/* Play/Pause */}
            <button
              onClick={togglePlay}
              className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
            >
              {isPlaying ? (
                <Pause className="w-5 h-5" />
              ) : (
                <Play className="w-5 h-5" />
              )}
            </button>

            {/* Skip forward */}
            <button
              onClick={() => skip(10)}
              className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
              title="Skip forward 10s"
            >
              <SkipForward className="w-4 h-4" />
            </button>

            {/* Time display */}
            <span className="text-sm text-slate-400 ml-2">
              {formatTime(currentPosition)} / {formatTime(duration)}
            </span>
          </div>

          <div className="flex items-center gap-2">
            {/* Playback rate */}
            <button
              onClick={changePlaybackRate}
              className={clsx(
                "px-2 py-1 text-xs font-medium rounded transition-colors",
                playbackRate !== 1
                  ? "bg-blue-600 text-white"
                  : "bg-slate-700 text-slate-300 hover:bg-slate-600"
              )}
            >
              {playbackRate}x
            </button>

            {/* Volume */}
            <button
              onClick={toggleMute}
              className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
            >
              {isMuted ? (
                <VolumeX className="w-4 h-4" />
              ) : (
                <Volume2 className="w-4 h-4" />
              )}
            </button>

            {/* Fullscreen */}
            <button
              onClick={toggleFullscreen}
              className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
            >
              <Maximize className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
