import { useRef, useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Play, Pause, Volume2, VolumeX, SkipBack, SkipForward, Music } from "lucide-react";
import clsx from "clsx";

interface AudioPlayerProps {
  filePath: string;
  currentTime?: number;
  onTimeUpdate?: (time: number) => void;
  title?: string;
}

export default function AudioPlayer({ filePath, currentTime, onTimeUpdate, title }: AudioPlayerProps) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const progressRef = useRef<HTMLDivElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [volume, setVolume] = useState(1);
  const [duration, setDuration] = useState(0);
  const [currentPosition, setCurrentPosition] = useState(0);
  const [playbackRate, setPlaybackRate] = useState(1);
  const [isLoaded, setIsLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Convert local file path to Tauri asset URL
  const audioUrl = convertFileSrc(filePath);

  // Seek to timestamp when currentTime prop changes
  useEffect(() => {
    if (currentTime !== undefined && audioRef.current && isLoaded) {
      audioRef.current.currentTime = currentTime;
      setCurrentPosition(currentTime);
    }
  }, [currentTime, isLoaded]);

  // Handle audio metadata loaded
  const handleLoadedMetadata = () => {
    if (audioRef.current) {
      setDuration(audioRef.current.duration);
      setIsLoaded(true);
      setError(null);
    }
  };

  // Handle time update
  const handleTimeUpdate = () => {
    if (audioRef.current) {
      const time = audioRef.current.currentTime;
      setCurrentPosition(time);
      onTimeUpdate?.(time);
    }
  };

  // Handle play/pause
  const togglePlay = () => {
    if (audioRef.current) {
      if (isPlaying) {
        audioRef.current.pause();
        setIsPlaying(false);
      } else {
        audioRef.current.play()
          .then(() => setIsPlaying(true))
          .catch((err) => {
            // Handle autoplay restrictions or other play errors
            console.warn("Audio play failed:", err);
            setError("Failed to play audio. This may be due to browser autoplay restrictions.");
          });
      }
    }
  };

  // Handle mute
  const toggleMute = () => {
    if (audioRef.current) {
      audioRef.current.muted = !isMuted;
      setIsMuted(!isMuted);
    }
  };

  // Handle volume change
  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newVolume = parseFloat(e.target.value);
    if (audioRef.current) {
      audioRef.current.volume = newVolume;
    }
    setVolume(newVolume);
    if (newVolume === 0) {
      setIsMuted(true);
    } else if (isMuted) {
      setIsMuted(false);
    }
  };

  // Handle seek
  const handleSeek = (e: React.MouseEvent<HTMLDivElement>) => {
    if (progressRef.current && audioRef.current) {
      const rect = progressRef.current.getBoundingClientRect();
      // Guard against division by zero
      if (rect.width === 0 || duration === 0) return;
      const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const newTime = percent * duration;
      if (!isFinite(newTime)) return;
      audioRef.current.currentTime = newTime;
      setCurrentPosition(newTime);
    }
  };

  // Handle playback rate
  const changePlaybackRate = () => {
    const rates = [0.5, 0.75, 1, 1.25, 1.5, 2];
    const currentIndex = rates.indexOf(playbackRate);
    const nextRate = rates[(currentIndex + 1) % rates.length];
    if (audioRef.current) {
      audioRef.current.playbackRate = nextRate;
    }
    setPlaybackRate(nextRate);
  };

  // Skip forward/backward
  const skip = (seconds: number) => {
    if (audioRef.current) {
      audioRef.current.currentTime = Math.max(0, Math.min(duration, audioRef.current.currentTime + seconds));
    }
  };

  // Handle error
  const handleError = () => {
    setError("Failed to load audio. The file may be corrupted or unsupported.");
  };

  // Format time
  const formatTime = (seconds: number) => {
    if (!isFinite(seconds)) return "0:00";
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
    <div className="flex flex-col h-full bg-gradient-to-br from-slate-800 to-slate-900">
      {/* Hidden audio element */}
      <audio
        ref={audioRef}
        src={audioUrl}
        onLoadedMetadata={handleLoadedMetadata}
        onTimeUpdate={handleTimeUpdate}
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onError={handleError}
      />

      {/* Visual display */}
      <div className="flex-1 flex flex-col items-center justify-center p-8">
        {/* Album art placeholder */}
        <div className="w-32 h-32 bg-gradient-to-br from-purple-600 to-blue-600 rounded-2xl flex items-center justify-center mb-6 shadow-xl">
          <Music className="w-16 h-16 text-white/80" />
        </div>

        {/* Title */}
        {title && (
          <h3 className="text-lg font-medium text-slate-200 mb-2 text-center truncate max-w-full">
            {title}
          </h3>
        )}

        {/* Time display */}
        <div className="text-sm text-slate-400">
          {formatTime(currentPosition)} / {formatTime(duration)}
        </div>

        {/* Loading indicator */}
        {!isLoaded && !error && (
          <div className="mt-4">
            <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full" />
          </div>
        )}
      </div>

      {/* Controls */}
      <div className="bg-slate-800/80 backdrop-blur border-t border-slate-700 p-4">
        {/* Progress bar */}
        <div
          ref={progressRef}
          className="h-2 bg-slate-600 rounded-full cursor-pointer mb-4 group"
          onClick={handleSeek}
        >
          <div
            className="h-full bg-gradient-to-r from-purple-500 to-blue-500 rounded-full relative"
            style={{ width: `${progressPercent}%` }}
          >
            <div className="absolute right-0 top-1/2 -translate-y-1/2 w-4 h-4 bg-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity shadow-lg" />
          </div>
        </div>

        {/* Control buttons */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1">
            {/* Skip back */}
            <button
              onClick={() => skip(-15)}
              className="p-3 hover:bg-slate-700 rounded-full transition-colors"
              title="Skip back 15s"
            >
              <SkipBack className="w-5 h-5" />
            </button>

            {/* Play/Pause */}
            <button
              onClick={togglePlay}
              className="p-4 bg-gradient-to-r from-purple-600 to-blue-600 hover:from-purple-500 hover:to-blue-500 rounded-full transition-all shadow-lg"
            >
              {isPlaying ? (
                <Pause className="w-6 h-6" />
              ) : (
                <Play className="w-6 h-6 ml-0.5" />
              )}
            </button>

            {/* Skip forward */}
            <button
              onClick={() => skip(15)}
              className="p-3 hover:bg-slate-700 rounded-full transition-colors"
              title="Skip forward 15s"
            >
              <SkipForward className="w-5 h-5" />
            </button>
          </div>

          <div className="flex items-center gap-3">
            {/* Playback rate */}
            <button
              onClick={changePlaybackRate}
              className={clsx(
                "px-2.5 py-1 text-xs font-medium rounded-full transition-colors",
                playbackRate !== 1
                  ? "bg-purple-600 text-white"
                  : "bg-slate-700 text-slate-300 hover:bg-slate-600"
              )}
            >
              {playbackRate}x
            </button>

            {/* Volume */}
            <div className="flex items-center gap-2">
              <button
                onClick={toggleMute}
                className="p-2 hover:bg-slate-700 rounded-lg transition-colors"
              >
                {isMuted || volume === 0 ? (
                  <VolumeX className="w-4 h-4" />
                ) : (
                  <Volume2 className="w-4 h-4" />
                )}
              </button>
              <input
                type="range"
                min="0"
                max="1"
                step="0.1"
                value={isMuted ? 0 : volume}
                onChange={handleVolumeChange}
                className="w-20 h-1 bg-slate-600 rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:rounded-full"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
