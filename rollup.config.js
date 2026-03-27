import resolve from '@rollup/plugin-node-resolve';

export default [
  {
    input: 'src/domain/learning/index.js',
    output: {
      file: 'dist/learning-domain.js',
      format: 'iife',
      name: 'LearningDomain',
    },
    plugins: [resolve()],
  },
  {
    input: 'src/domain/challenge/index.js',
    output: {
      file: 'dist/challenge-domain.js',
      format: 'iife',
      name: 'ChallengeDomain',
    },
    plugins: [resolve()],
  },
  {
    input: 'src/domain/economy/index.js',
    output: {
      file: 'dist/economy-domain.js',
      format: 'iife',
      name: 'EconomyDomain',
    },
    plugins: [resolve()],
  },
  {
    input: 'src/infrastructure/speech-recognition.js',
    output: {
      file: 'dist/speech-recognition.js',
      format: 'iife',
      name: 'SpeechRecognitionModule',
    },
    plugins: [resolve()],
  },
];
