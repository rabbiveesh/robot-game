// Learning domain — public API

export { createProfile, learnerReducer } from './learner-profile.js';
export { createWindow, pushEntry, accuracy, avgResponseTime, consecutiveWrong, operationAccuracy, accuracyAtBand, accuracyAboveBand } from './rolling-window.js';
export { createOperationStats, recordOperation } from './operation-stats.js';
export { generateChallenge, bandDistribution, sampleFromDistribution, classifyAddition, classifySubtraction, classifyMultiplication, classifyDivision, classifyBond, extractFeatures } from './challenge-generator.js';
export { generateIntakeQuestion, processIntakeResults, nextIntakeBand } from './intake-assessor.js';
export { detectFrustration } from './frustration-detector.js';
