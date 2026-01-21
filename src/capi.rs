use std::{
    ffi::CStr,
    ffi::c_void,
    os::raw::c_char,
    panic::{catch_unwind, AssertUnwindSafe},
};

use rosu_map::section::general::GameMode;

use crate::{
    any::{DifficultyAttributes, PerformanceAttributes, ScoreState},
    Beatmap, Difficulty, GradualPerformance, Performance,
};

/// Error codes returned by the C API.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RosuPpError {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    IoError = 3,
    Panic = 4,
    TooSuspicious = 5,
    EndOfStream = 6,
}

/// Game mode of the parsed beatmap.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RosuPpGameMode {
    Osu = 0,
    Taiko = 1,
    Catch = 2,
    Mania = 3,
}

impl Default for RosuPpGameMode {
    fn default() -> Self {
        Self::Osu
    }
}

/// Reason why a beatmap was flagged as suspicious.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RosuPpSuspicion {
    None = 0,
    Density = 1,
    Length = 2,
    ObjectCount = 3,
    RedFlag = 4,
    SliderPositions = 5,
    SliderRepeats = 6,
    Unknown = 255,
}

/// C representation of [`crate::any::ScoreState`].
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RosuPpScoreState {
    pub max_combo: u32,
    pub osu_large_tick_hits: u32,
    pub osu_small_tick_hits: u32,
    pub slider_end_hits: u32,
    pub n_geki: u32,
    pub n_katu: u32,
    pub n300: u32,
    pub n100: u32,
    pub n50: u32,
    pub misses: u32,
}

impl From<RosuPpScoreState> for ScoreState {
    fn from(state: RosuPpScoreState) -> Self {
        Self {
            max_combo: state.max_combo,
            osu_large_tick_hits: state.osu_large_tick_hits,
            osu_small_tick_hits: state.osu_small_tick_hits,
            slider_end_hits: state.slider_end_hits,
            n_geki: state.n_geki,
            n_katu: state.n_katu,
            n300: state.n300,
            n100: state.n100,
            n50: state.n50,
            misses: state.misses,
        }
    }
}

/// Result of a difficulty calculation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RosuPpDifficultyAttributes {
    pub stars: f64,
    pub max_combo: u32,
    pub mode: RosuPpGameMode,
}

/// Result of a combined difficulty + performance calculation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RosuPpPerformanceAttributes {
    pub pp: f64,
    pub stars: f64,
    pub max_combo: u32,
    pub mode: RosuPpGameMode,
}

fn mode_into_c(mode: GameMode) -> RosuPpGameMode {
    match mode {
        GameMode::Osu => RosuPpGameMode::Osu,
        GameMode::Taiko => RosuPpGameMode::Taiko,
        GameMode::Catch => RosuPpGameMode::Catch,
        GameMode::Mania => RosuPpGameMode::Mania,
    }
}

fn suspicion_into_c(sus: crate::model::beatmap::TooSuspicious) -> RosuPpSuspicion {
    use crate::model::beatmap::TooSuspicious;

    match sus {
        TooSuspicious::Density => RosuPpSuspicion::Density,
        TooSuspicious::Length => RosuPpSuspicion::Length,
        TooSuspicious::ObjectCount => RosuPpSuspicion::ObjectCount,
        TooSuspicious::RedFlag => RosuPpSuspicion::RedFlag,
        TooSuspicious::SliderPositions => RosuPpSuspicion::SliderPositions,
        TooSuspicious::SliderRepeats => RosuPpSuspicion::SliderRepeats,
        _ => RosuPpSuspicion::Unknown,
    }
}

fn difficulty_mode(attrs: &DifficultyAttributes) -> RosuPpGameMode {
    match attrs {
        DifficultyAttributes::Osu(_) => RosuPpGameMode::Osu,
        DifficultyAttributes::Taiko(_) => RosuPpGameMode::Taiko,
        DifficultyAttributes::Catch(_) => RosuPpGameMode::Catch,
        DifficultyAttributes::Mania(_) => RosuPpGameMode::Mania,
    }
}

fn performance_mode(attrs: &PerformanceAttributes) -> RosuPpGameMode {
    match attrs {
        PerformanceAttributes::Osu(_) => RosuPpGameMode::Osu,
        PerformanceAttributes::Taiko(_) => RosuPpGameMode::Taiko,
        PerformanceAttributes::Catch(_) => RosuPpGameMode::Catch,
        PerformanceAttributes::Mania(_) => RosuPpGameMode::Mania,
    }
}

fn calculate_performance(
    map: &Beatmap,
    mods: u32,
    accuracy: f64,
    combo: u32,
    misses: u32,
) -> PerformanceAttributes {
    Performance::new(map)
        .mods(mods)
        .accuracy(accuracy)
        .combo(combo)
        .misses(misses)
        .calculate()
}

/// Convert an error code into a static, NUL-terminated string.
#[no_mangle]
pub extern "C" fn rosu_pp_error_str(err: i32) -> *const c_char {
    match err {
        x if x == RosuPpError::Ok as i32 => b"Ok\0".as_ptr(),
        x if x == RosuPpError::NullPointer as i32 => b"NullPointer\0".as_ptr(),
        x if x == RosuPpError::InvalidUtf8 as i32 => b"InvalidUtf8\0".as_ptr(),
        x if x == RosuPpError::IoError as i32 => b"IoError\0".as_ptr(),
        x if x == RosuPpError::Panic as i32 => b"Panic\0".as_ptr(),
        x if x == RosuPpError::TooSuspicious as i32 => b"TooSuspicious\0".as_ptr(),
        x if x == RosuPpError::EndOfStream as i32 => b"EndOfStream\0".as_ptr(),
        _ => b"Unknown\0".as_ptr(),
    }
    .cast::<c_char>()
}

/// Create a new empty score state.
#[no_mangle]
pub extern "C" fn rosu_pp_score_state_new() -> RosuPpScoreState {
    RosuPpScoreState::default()
}

/// Parse a beatmap from a `.osu` file path.
///
/// # Safety
/// - `path` must be a valid, NUL-terminated UTF-8 string.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_beatmap_from_path(
    path: *const c_char,
    out: *mut *mut c_void,
) -> RosuPpError {
    if path.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let path = CStr::from_ptr(path)
            .to_str()
            .map_err(|_| RosuPpError::InvalidUtf8)?;

        let map = Beatmap::from_path(path).map_err(|_| RosuPpError::IoError)?;
        out.write(Box::into_raw(Box::new(map)).cast::<c_void>());

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Parse a beatmap from an in-memory `.osu` file.
///
/// # Safety
/// - If `len != 0`, `bytes` must be valid for reads of `len` bytes.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_beatmap_from_bytes(
    bytes: *const u8,
    len: usize,
    out: *mut *mut c_void,
) -> RosuPpError {
    if out.is_null() || (bytes.is_null() && len != 0) {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let bytes = if len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(bytes, len)
        };

        let map = Beatmap::from_bytes(bytes).map_err(|_| RosuPpError::IoError)?;
        out.write(Box::into_raw(Box::new(map)).cast::<c_void>());

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Free a beatmap created through `rosu_pp_beatmap_from_*`.
///
/// # Safety
/// - `map` must either be `NULL` or a pointer returned by `rosu_pp_beatmap_from_*`.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_beatmap_free(map: *mut c_void) {
    if map.is_null() {
        return;
    }

    let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(map.cast::<Beatmap>()))));
}

/// Get a beatmap's mode.
///
/// # Safety
/// - `map` must be a valid pointer returned by `rosu_pp_beatmap_from_*`.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_beatmap_mode(map: *const c_void, out: *mut RosuPpGameMode) -> RosuPpError {
    if map.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let map = &*map.cast::<Beatmap>();
        out.write(mode_into_c(map.mode));
        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Check whether a beatmap appears too suspicious for further calculation.
///
/// # Safety
/// - `map` must be a valid pointer returned by `rosu_pp_beatmap_from_*`.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_beatmap_check_suspicion(
    map: *const c_void,
    out: *mut RosuPpSuspicion,
) -> RosuPpError {
    if map.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let map = &*map.cast::<Beatmap>();

        match map.check_suspicion() {
            Ok(()) => {
                out.write(RosuPpSuspicion::None);
                Ok(RosuPpError::Ok)
            }
            Err(sus) => {
                out.write(suspicion_into_c(sus));
                Ok(RosuPpError::TooSuspicious)
            }
        }
    })) {
        Ok(Ok(err)) => err,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Calculate difficulty attributes from a parsed beatmap.
///
/// # Safety
/// - `map` must be a valid pointer returned by `rosu_pp_beatmap_from_*`.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_difficulty_calculate(
    map: *const c_void,
    mods: u32,
    out: *mut *mut c_void,
) -> RosuPpError {
    if map.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let map = &*map.cast::<Beatmap>();
        let attrs = Difficulty::new().mods(mods).calculate(map);
        out.write(Box::into_raw(Box::new(attrs)).cast::<c_void>());
        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Free difficulty attributes created through `rosu_pp_difficulty_calculate`.
///
/// # Safety
/// - `attrs` must either be `NULL` or a pointer returned by `rosu_pp_difficulty_calculate`.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_difficulty_attrs_free(attrs: *mut c_void) {
    if attrs.is_null() {
        return;
    }

    let _ = catch_unwind(AssertUnwindSafe(|| {
        drop(Box::from_raw(attrs.cast::<DifficultyAttributes>()));
    }));
}

/// Extract common values from difficulty attributes.
///
/// # Safety
/// - `attrs` must be a valid pointer returned by `rosu_pp_difficulty_calculate`.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_difficulty_attrs_values(
    attrs: *const c_void,
    out: *mut RosuPpDifficultyAttributes,
) -> RosuPpError {
    if attrs.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let attrs = &*attrs.cast::<DifficultyAttributes>();

        out.write(RosuPpDifficultyAttributes {
            stars: attrs.stars(),
            max_combo: attrs.max_combo(),
            mode: difficulty_mode(attrs),
        });

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Calculate performance attributes from previously calculated difficulty attributes.
///
/// # Safety
/// - `difficulty` must be a valid pointer returned by `rosu_pp_difficulty_calculate`.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_performance_calculate(
    difficulty: *const c_void,
    mods: u32,
    accuracy: f64,
    combo: u32,
    misses: u32,
    out: *mut *mut c_void,
) -> RosuPpError {
    if difficulty.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let difficulty = (&*difficulty.cast::<DifficultyAttributes>()).clone();
        let attrs = Performance::new(difficulty)
            .mods(mods)
            .combo(combo)
            .accuracy(accuracy)
            .misses(misses)
            .calculate();

        out.write(Box::into_raw(Box::new(attrs)).cast::<c_void>());

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Free performance attributes created through `rosu_pp_performance_calculate`.
///
/// # Safety
/// - `attrs` must either be `NULL` or a pointer returned by `rosu_pp_performance_calculate`.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_performance_attrs_free(attrs: *mut c_void) {
    if attrs.is_null() {
        return;
    }

    let _ = catch_unwind(AssertUnwindSafe(|| {
        drop(Box::from_raw(attrs.cast::<PerformanceAttributes>()));
    }));
}

/// Extract common values from performance attributes.
///
/// # Safety
/// - `attrs` must be a valid pointer returned by `rosu_pp_performance_calculate`.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_performance_attrs_values(
    attrs: *const c_void,
    out: *mut RosuPpPerformanceAttributes,
) -> RosuPpError {
    if attrs.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let attrs = &*attrs.cast::<PerformanceAttributes>();

        out.write(RosuPpPerformanceAttributes {
            pp: attrs.pp(),
            stars: attrs.stars(),
            max_combo: attrs.max_combo(),
            mode: performance_mode(attrs),
        });

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Calculate the maximum pp for the given performance attributes.
///
/// This is equivalent to `perf_attrs.performance().mods(mods).calculate().pp()` in Rust.
///
/// # Safety
/// - `attrs` must be a valid pointer returned by `rosu_pp_performance_calculate`.
/// - `out_pp` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_performance_attrs_max_pp(
    attrs: *const c_void,
    mods: u32,
    out_pp: *mut f64,
) -> RosuPpError {
    if attrs.is_null() || out_pp.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let attrs = (&*attrs.cast::<PerformanceAttributes>()).clone();
        let pp = attrs.performance().mods(mods).calculate().pp();
        out_pp.write(pp);
        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Create a gradual performance calculator.
///
/// `clock_rate <= 0.0` means "use the default clock rate based on mods".
///
/// # Safety
/// - `map` must be a valid pointer returned by `rosu_pp_beatmap_from_*`.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_gradual_performance_new(
    map: *const c_void,
    mods: u32,
    clock_rate: f64,
    out: *mut *mut c_void,
) -> RosuPpError {
    if map.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let map = &*map.cast::<Beatmap>();
        let mut difficulty = Difficulty::new().mods(mods);
        if clock_rate > 0.0 {
            difficulty = difficulty.clock_rate(clock_rate);
        }

        let gradual = GradualPerformance::new(difficulty, map);
        out.write(Box::into_raw(Box::new(gradual)).cast::<c_void>());

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Free a gradual performance calculator created through `rosu_pp_gradual_performance_new`.
///
/// # Safety
/// - `gradual` must either be `NULL` or a pointer returned by `rosu_pp_gradual_performance_new`.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_gradual_performance_free(gradual: *mut c_void) {
    if gradual.is_null() {
        return;
    }

    let _ = catch_unwind(AssertUnwindSafe(|| {
        drop(Box::from_raw(gradual.cast::<GradualPerformance>()));
    }));
}

/// Process the next hitobject and calculate the current performance attributes.
///
/// # Safety
/// - `gradual` must be a valid pointer returned by `rosu_pp_gradual_performance_new`.
/// - `state` and `out` must be valid pointers to readable/writable memory respectively.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_gradual_performance_next(
    gradual: *mut c_void,
    state: *const RosuPpScoreState,
    out: *mut RosuPpPerformanceAttributes,
) -> RosuPpError {
    if gradual.is_null() || state.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let gradual = &mut *gradual.cast::<GradualPerformance>();
        let state = (*state).into();

        match gradual.next(state) {
            Some(attrs) => {
                out.write(RosuPpPerformanceAttributes {
                    pp: attrs.pp(),
                    stars: attrs.stars(),
                    max_combo: attrs.max_combo(),
                    mode: performance_mode(&attrs),
                });

                Ok(RosuPpError::Ok)
            }
            None => Ok(RosuPpError::EndOfStream),
        }
    })) {
        Ok(Ok(err)) => err,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Process all remaining hitobjects and calculate the final performance attributes.
///
/// # Safety
/// - `gradual` must be a valid pointer returned by `rosu_pp_gradual_performance_new`.
/// - `state` and `out` must be valid pointers to readable/writable memory respectively.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_gradual_performance_last(
    gradual: *mut c_void,
    state: *const RosuPpScoreState,
    out: *mut RosuPpPerformanceAttributes,
) -> RosuPpError {
    if gradual.is_null() || state.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let gradual = &mut *gradual.cast::<GradualPerformance>();
        let state = (*state).into();

        match gradual.last(state) {
            Some(attrs) => {
                out.write(RosuPpPerformanceAttributes {
                    pp: attrs.pp(),
                    stars: attrs.stars(),
                    max_combo: attrs.max_combo(),
                    mode: performance_mode(&attrs),
                });

                Ok(RosuPpError::Ok)
            }
            None => Ok(RosuPpError::EndOfStream),
        }
    })) {
        Ok(Ok(err)) => err,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Calculate performance attributes from a `.osu` file path.
///
/// # Safety
/// - `path` must be a valid, NUL-terminated UTF-8 string.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_performance_from_path(
    path: *const c_char,
    mods: u32,
    accuracy: f64,
    combo: u32,
    misses: u32,
    out: *mut RosuPpPerformanceAttributes,
) -> RosuPpError {
    if path.is_null() || out.is_null() {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let path = CStr::from_ptr(path)
            .to_str()
            .map_err(|_| RosuPpError::InvalidUtf8)?;

        let map = Beatmap::from_path(path).map_err(|_| RosuPpError::IoError)?;
        let mode = mode_into_c(map.mode);
        let attrs = calculate_performance(&map, mods, accuracy, combo, misses);

        out.write(RosuPpPerformanceAttributes {
            pp: attrs.pp(),
            stars: attrs.stars(),
            max_combo: attrs.max_combo(),
            mode,
        });

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}

/// Calculate performance attributes from an in-memory `.osu` file.
///
/// # Safety
/// - If `len != 0`, `bytes` must be valid for reads of `len` bytes.
/// - `out` must be a valid pointer to writable memory.
#[no_mangle]
pub unsafe extern "C" fn rosu_pp_performance_from_bytes(
    bytes: *const u8,
    len: usize,
    mods: u32,
    accuracy: f64,
    combo: u32,
    misses: u32,
    out: *mut RosuPpPerformanceAttributes,
) -> RosuPpError {
    if out.is_null() || (bytes.is_null() && len != 0) {
        return RosuPpError::NullPointer;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let bytes = if len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(bytes, len)
        };
        let map = Beatmap::from_bytes(bytes).map_err(|_| RosuPpError::IoError)?;
        let mode = mode_into_c(map.mode);
        let attrs = calculate_performance(&map, mods, accuracy, combo, misses);

        out.write(RosuPpPerformanceAttributes {
            pp: attrs.pp(),
            stars: attrs.stars(),
            max_combo: attrs.max_combo(),
            mode,
        });

        Ok(())
    })) {
        Ok(Ok(())) => RosuPpError::Ok,
        Ok(Err(err)) => err,
        Err(_) => RosuPpError::Panic,
    }
}
