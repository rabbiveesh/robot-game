import resolve from '@rollup/plugin-node-resolve';

export default [
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
