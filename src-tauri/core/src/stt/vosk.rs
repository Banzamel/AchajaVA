//! Vosk, the offline recognizer of the control words, loaded at run time from the libvosk.dll
//! shipped in resources\vosk (with its MinGW runtime DLLs next to it). Loading it late keeps the
//! app working (window, settings, mail) even when the library or a model is missing.

use std::ffi::{c_char, c_float, c_int, c_short, c_void, CStr, CString};
use std::path::Path;
use std::sync::Arc;

use libloading::{Library, Symbol};

use crate::error::{AppError, Result};

type ModelNew = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type Free = unsafe extern "C" fn(*mut c_void);
type RecNew = unsafe extern "C" fn(*mut c_void, c_float) -> *mut c_void;
type RecNewGrm = unsafe extern "C" fn(*mut c_void, c_float, *const c_char) -> *mut c_void;
type SetWords = unsafe extern "C" fn(*mut c_void, c_int);
type Accept = unsafe extern "C" fn(*mut c_void, *const c_short, c_int) -> c_int;
type Text = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type LogLevel = unsafe extern "C" fn(c_int);

pub struct Vosk {
    lib: Library,
}

fn err(e: impl std::fmt::Display) -> AppError {
    AppError::Audio(format!("Vosk: {e}"))
}

impl Vosk {
    pub fn load(dll: &Path) -> Result<Arc<Vosk>> {
        #[cfg(windows)]
        // SAFETY: loading libvosk runs its initializers; the flag makes Windows find the
        // dependent DLLs in the same folder.
        let lib = unsafe {
            libloading::os::windows::Library::load_with_flags(dll, libloading::os::windows::LOAD_WITH_ALTERED_SEARCH_PATH)
                .map(Library::from)
        };
        #[cfg(not(windows))]
        // SAFETY: as above, for the Linux library in tests.
        let lib = unsafe { Library::new(dll) };
        let vosk = Vosk { lib: lib.map_err(|e| err(format!("{}: {e}", dll.display())))? };
        if let Ok(set) = vosk.sym::<LogLevel>(b"vosk_set_log_level\0") {
            // SAFETY: a plain C call with an int.
            unsafe { set(-1) };
        }
        Ok(Arc::new(vosk))
    }

    fn sym<T>(&self, name: &[u8]) -> Result<Symbol<'_, T>> {
        // SAFETY: the types above follow vosk_api.h of Vosk 0.3.45.
        unsafe { self.lib.get::<T>(name).map_err(err) }
    }
}

pub struct Model {
    vosk: Arc<Vosk>,
    ptr: *mut c_void,
}

impl Model {
    pub fn new(vosk: &Arc<Vosk>, dir: &Path) -> Result<Model> {
        let path = CString::new(dir.to_string_lossy().as_bytes()).map_err(err)?;
        let new = vosk.sym::<ModelNew>(b"vosk_model_new\0")?;
        // SAFETY: a NUL-terminated path; Vosk returns NULL when the model cannot be read.
        let ptr = unsafe { new(path.as_ptr()) };
        if ptr.is_null() {
            return Err(err(format!("model {} could not be loaded", dir.display())));
        }
        Ok(Model { vosk: vosk.clone(), ptr })
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        if let Ok(free) = self.vosk.sym::<Free>(b"vosk_model_free\0") {
            // SAFETY: the pointer came from vosk_model_new and is freed once.
            unsafe { free(self.ptr) };
        }
    }
}

pub struct Recognizer {
    vosk: Arc<Vosk>,
    ptr: *mut c_void,
}

impl Recognizer {
    /// grammar: a JSON list of phrases (with "[unk]"), or None for the full vocabulary.
    pub fn new(model: &Model, rate: f32, grammar: Option<&str>) -> Result<Recognizer> {
        let vosk = model.vosk.clone();
        let ptr = match grammar {
            Some(g) => {
                let g = CString::new(g).map_err(err)?;
                let f = vosk.sym::<RecNewGrm>(b"vosk_recognizer_new_grm\0")?;
                // SAFETY: valid model pointer and NUL-terminated grammar.
                unsafe { f(model.ptr, rate, g.as_ptr()) }
            }
            None => {
                let f = vosk.sym::<RecNew>(b"vosk_recognizer_new\0")?;
                // SAFETY: valid model pointer.
                unsafe { f(model.ptr, rate) }
            }
        };
        if ptr.is_null() {
            return Err(err("recognizer could not be created"));
        }
        Ok(Recognizer { vosk, ptr })
    }

    pub fn set_words(&self, on: bool) {
        if let Ok(f) = self.vosk.sym::<SetWords>(b"vosk_recognizer_set_words\0") {
            // SAFETY: valid recognizer pointer.
            unsafe { f(self.ptr, c_int::from(on)) };
        }
    }

    /// True when an utterance ended (read it with result()).
    pub fn accept(&self, samples: &[i16]) -> bool {
        match self.vosk.sym::<Accept>(b"vosk_recognizer_accept_waveform_s\0") {
            // SAFETY: the slice is valid for its length in samples.
            Ok(f) => unsafe { f(self.ptr, samples.as_ptr(), samples.len() as c_int) == 1 },
            Err(_) => false,
        }
    }

    fn text(&self, name: &[u8]) -> String {
        let Ok(f) = self.vosk.sym::<Text>(name) else { return "{}".into() };
        // SAFETY: Vosk returns a NUL-terminated string owned by the recognizer, valid until the
        // next call on it; it is copied at once.
        unsafe {
            let p = f(self.ptr);
            if p.is_null() {
                "{}".into()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }

    pub fn result(&self) -> String {
        self.text(b"vosk_recognizer_result\0")
    }

    pub fn partial(&self) -> String {
        self.text(b"vosk_recognizer_partial_result\0")
    }

    pub fn reset(&self) {
        if let Ok(f) = self.vosk.sym::<Free>(b"vosk_recognizer_reset\0") {
            // SAFETY: valid recognizer pointer (reset has the same signature as free).
            unsafe { f(self.ptr) };
        }
    }
}

impl Drop for Recognizer {
    fn drop(&mut self) {
        if let Ok(free) = self.vosk.sym::<Free>(b"vosk_recognizer_free\0") {
            // SAFETY: the pointer came from vosk_recognizer_new* and is freed once.
            unsafe { free(self.ptr) };
        }
    }
}
