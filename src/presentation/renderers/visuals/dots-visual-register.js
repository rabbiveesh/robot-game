// Register the existing renderDotVisual (defined in dialogue.js) with the visual registry
if (typeof registerVisual === 'function' && typeof renderDotVisual === 'function') {
  registerVisual('dots', {
    name: 'Counting Dots',
    description: 'Individual dots. Count them up.',
    operations: ['add', 'sub'],
    bandRange: [1, 4],
    craStage: 'concrete',
  }, renderDotVisual);
}
