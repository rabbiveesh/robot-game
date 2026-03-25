import { describe, it, expect } from 'vitest';
import { detectFrustration } from '../../../src/domain/learning/frustration-detector.js';
import { createWindow } from '../../../src/domain/learning/rolling-window.js';

describe('frustration-detector', () => {
  it('detects high frustration after 3 consecutive wrong', () => {
    const w = createWindow([
      { correct: true },
      { correct: false },
      { correct: false },
      { correct: false },
    ]);
    const result = detectFrustration(w);
    expect(result.level).toBe('high');
    expect(result.recommendation).toBe('drop_band');
  });

  it('detects high frustration on rapid clicking', () => {
    const w = createWindow([{ correct: true }]);
    const behaviors = [{ signal: 'rapid_clicking' }];
    const result = detectFrustration(w, behaviors);
    expect(result.level).toBe('high');
    expect(result.recommendation).toBe('drop_band');
  });

  it('detects high frustration on low accuracy', () => {
    // 5 entries, only 1 correct = 20% accuracy
    const w = createWindow([
      { correct: true },
      { correct: false },
      { correct: false },
      { correct: false },
      { correct: true }, // end on correct so consecutive wrong doesn't trigger first
    ]);
    // Actually need accuracy < 40% with 5+ entries
    const w2 = createWindow([
      { correct: false },
      { correct: false },
      { correct: false },
      { correct: true },
      { correct: false },
      { correct: true }, // 2/6 = 33%
    ]);
    const result = detectFrustration(w2);
    expect(result.level).toBe('high');
    expect(result.recommendation).toBe('switch_to_chat');
  });

  it('detects mild frustration on long idle after wrong', () => {
    const w = createWindow([
      { correct: true },
      { correct: false, responseTimeMs: 20000 },
    ]);
    const result = detectFrustration(w);
    expect(result.level).toBe('mild');
    expect(result.recommendation).toBe('encourage');
  });

  it('detects mild frustration on choosing easier path twice', () => {
    const w = createWindow([{ correct: true }]);
    const behaviors = [
      { signal: 'chose_easier_path' },
      { signal: 'chose_easier_path' },
    ];
    const result = detectFrustration(w, behaviors);
    expect(result.level).toBe('mild');
    expect(result.recommendation).toBe('offer_easier_path');
  });

  it('returns none when accuracy is healthy', () => {
    const w = createWindow([
      { correct: true },
      { correct: true },
      { correct: true },
      { correct: false, responseTimeMs: 3000 },
      { correct: true },
    ]);
    const result = detectFrustration(w);
    expect(result.level).toBe('none');
    expect(result.recommendation).toBe('continue');
  });

  it('returns none for empty window', () => {
    const result = detectFrustration(createWindow());
    expect(result.level).toBe('none');
    expect(result.recommendation).toBe('continue');
  });

  it('high frustration from consecutive wrong takes priority over mild', () => {
    const w = createWindow([
      { correct: false, responseTimeMs: 20000 },
      { correct: false, responseTimeMs: 20000 },
      { correct: false, responseTimeMs: 20000 },
    ]);
    const result = detectFrustration(w);
    expect(result.level).toBe('high');
  });
});
