// ============================================================================
// Adapters Module
// ============================================================================
// External service integrations:
// - FFmpeg/ffprobe for audio extraction and validation
// - Groq API for speech-to-text and postprocessing
// - OS Keyring for secure API key storage
// - Filesystem utilities (atomic writes, hashing)

#![allow(dead_code)] // Many adapter fields are used in future phases

pub mod ffmpeg;
pub mod groq;
pub mod keyring;
// TODO: pub mod fs;
