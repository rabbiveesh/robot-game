import resolve from '@rollup/plugin-node-resolve';

export default {
  input: 'src/domain/learning/index.js',
  output: {
    file: 'dist/learning-domain.js',
    format: 'iife',
    name: 'LearningDomain',
  },
  plugins: [resolve()],
};
