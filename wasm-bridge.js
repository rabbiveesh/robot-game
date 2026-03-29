// wasm-bridge.js — Loads WASM domain and exposes it as window.WasmDomain
// Must be loaded and awaited BEFORE adapter.js

(async function () {
  try {
    const wasm = await import('./dist/wasm/robot_buddy_domain.js');
    await wasm.default();

    // Thin wrappers: JSON in/out, matching the shape the adapter expects
    window.WasmDomain = {
      // ── Profile ──
      createProfile(overrides) {
        if (overrides && Object.keys(overrides).length > 0) {
          // Convert JS camelCase to Rust snake_case
          const mapped = {};
          if (overrides.mathBand != null) mapped.math_band = overrides.mathBand;
          if (overrides.spreadWidth != null) mapped.spread_width = overrides.spreadWidth;
          if (overrides.pace != null) mapped.pace = overrides.pace;
          if (overrides.scaffolding != null) mapped.scaffolding = overrides.scaffolding;
          if (overrides.promoteThreshold != null) mapped.promote_threshold = overrides.promoteThreshold;
          if (overrides.stretchThreshold != null) mapped.stretch_threshold = overrides.stretchThreshold;
          if (overrides.textSpeed != null) mapped.text_speed = overrides.textSpeed;
          if (overrides.intakeCompleted != null) mapped.intake_completed = overrides.intakeCompleted;
          return JSON.parse(wasm.create_profile_with_overrides(JSON.stringify(mapped)));
        }
        return JSON.parse(wasm.create_profile());
      },

      learnerReducer(state, event) {
        return JSON.parse(wasm.learner_reducer(JSON.stringify(state), JSON.stringify(event)));
      },

      // ── Challenge Generator ──
      generateChallenge(profile, _rng) {
        // RNG: use a random seed (Rust creates its own PRNG from the seed)
        const seed = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
        const challengeProfile = {
          math_band: profile.math_band ?? profile.mathBand,
          spread_width: profile.spread_width ?? profile.spreadWidth ?? 0.5,
          operation_stats: profile.operation_stats ?? profile.operationStats,
        };
        return JSON.parse(wasm.generate_challenge(JSON.stringify(challengeProfile), seed));
      },

      // ── Frustration ──
      detectFrustration(window, behaviors) {
        return JSON.parse(wasm.detect_frustration(JSON.stringify(window), JSON.stringify(behaviors)));
      },

      // ── Intake ──
      generateIntakeQuestion(currentBand, questionIndex, _rng) {
        const seed = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
        return JSON.parse(wasm.generate_intake_question(currentBand, questionIndex, seed));
      },

      processIntakeResults(answers, configuredBand) {
        return JSON.parse(wasm.process_intake_results(JSON.stringify(answers), configuredBand ?? -1));
      },

      nextIntakeBand: wasm.next_intake_band,

      // ── Challenge Lifecycle ──
      challengeReducer(state, action) {
        return JSON.parse(wasm.challenge_reducer(JSON.stringify(state), JSON.stringify(action)));
      },

      // ── Helpers (computed from state, not WASM) ──
      accuracy(window) {
        if (!window || !window.entries || window.entries.length === 0) return 0;
        const correct = window.entries.filter(e => e.correct).length;
        return correct / window.entries.length;
      },

      createWindow(entries) {
        return { entries: entries || [], max_size: 20 };
      },
    };

    console.log('[WASM] Domain loaded — all modules active');
  } catch (e) {
    document.body.innerHTML = `<div style="color:#F44336;padding:40px;font-family:monospace;">
      <h1>WASM domain failed to load</h1>
      <p>${e.message}</p>
      <p>The game requires HTTPS or localhost. Try: <code>npx serve .</code></p>
    </div>`;
    throw e;
  }
})();
