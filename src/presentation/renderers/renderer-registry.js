// renderer-registry.js — Registry for challenge renderers

const RENDERER_REGISTRY = {};

function registerRenderer(id, meta, createFn) {
  RENDERER_REGISTRY[id] = Object.freeze({
    id,
    name: meta.name,
    description: meta.description,
    createFn,
  });
}

function getRenderer(id) {
  return RENDERER_REGISTRY[id] || null;
}

function getAllRenderers() {
  return Object.values(RENDERER_REGISTRY);
}
