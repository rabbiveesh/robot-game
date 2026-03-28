// visual-registry.js — Registry for visualization methods
// The "can't forget" pattern: if it's not registered, the game can't use it.

const VISUAL_REGISTRY = {};

function registerVisual(id, meta, renderFn) {
  VISUAL_REGISTRY[id] = Object.freeze({
    id,
    name: meta.name,
    description: meta.description,
    operations: meta.operations,
    bandRange: meta.bandRange,
    craStage: meta.craStage,
    render: renderFn,
  });
}

function getVisual(id) {
  return VISUAL_REGISTRY[id] || null;
}

function getAllVisuals() {
  return Object.values(VISUAL_REGISTRY);
}
