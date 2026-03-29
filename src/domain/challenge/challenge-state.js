// challenge-state.js — Challenge lifecycle state machine
// Pure domain. No browser deps. Single reducer for the entire lifecycle.

function resetVoice() {
  return Object.freeze({
    listening: false,
    confirming: false,
    confirmNumber: null,
    retries: 0,
    lastResult: null,
    text: null,
  });
}

const DEFAULT_RENDER_HINT = Object.freeze({
  craStage: 'abstract',
  answerMode: 'choice',
  interactionType: 'quiz',
});

export function createChallengeState(challenge, context) {
  return Object.freeze({
    phase: 'presented',

    challenge,
    context,
    attempts: 0,
    maxAttempts: 2,
    correct: null,

    question: Object.freeze({
      display: challenge.displayText || challenge.question,
      speech: challenge.speechText || challenge.question,
    }),
    feedback: null,
    reward: null,

    renderHint: Object.freeze(context.renderHint || DEFAULT_RENDER_HINT),

    hintUsed: false,
    hintLevel: 0,
    toldMe: false,

    voice: resetVoice(),
  });
}

export function challengeReducer(state, action) {
  switch (action.type) {
    case 'ANSWER_SUBMITTED': {
      const correct = action.answer === state.challenge.correctAnswer;
      const attempts = state.attempts + 1;

      if (correct) {
        return Object.freeze({
          ...state,
          phase: 'complete',
          correct: true,
          attempts,
          reward: Object.freeze({ type: 'dum_dum', amount: 1 }),
          feedback: Object.freeze({
            display: 'Amazing! You got it!',
            speech: 'Amazing! You got it!',
          }),
          voice: resetVoice(),
        });
      }

      if (attempts >= state.maxAttempts) {
        return Object.freeze({
          ...state,
          phase: 'teaching',
          correct: false,
          attempts,
          reward: null,
          feedback: Object.freeze({
            display: "Let's figure it out together!",
            speech: "Let's figure it out together!",
          }),
          voice: resetVoice(),
        });
      }

      return Object.freeze({
        ...state,
        phase: 'feedback',
        attempts,
        feedback: Object.freeze({
          display: 'Hmm, not quite! Try again!',
          speech: 'Hmm, not quite! Try again!',
        }),
      });
    }

    case 'RETRY': {
      return Object.freeze({
        ...state,
        phase: 'presented',
        feedback: null,
      });
    }

    case 'TEACHING_COMPLETE': {
      return Object.freeze({
        ...state,
        phase: 'complete',
      });
    }

    case 'SHOW_ME': {
      const currentCra = state.renderHint.craStage;
      const lowerCra = currentCra === 'abstract' ? 'representational'
        : currentCra === 'representational' ? 'concrete'
          : 'concrete';
      // Always set hintUsed — even at concrete, the kid asked for help
      return Object.freeze({
        ...state,
        renderHint: Object.freeze({ ...state.renderHint, craStage: lowerCra }),
        hintUsed: true,
        hintLevel: state.hintLevel + 1,
      });
    }

    case 'TELL_ME': {
      return Object.freeze({
        ...state,
        phase: 'teaching',
        toldMe: true,
        reward: null,
        feedback: Object.freeze({
          display: `The answer is ${state.challenge.correctAnswer}!`,
          speech: `The answer is ${state.challenge.correctAnswer}!`,
        }),
        renderHint: Object.freeze({ ...state.renderHint, craStage: 'concrete' }),
      });
    }

    case 'VOICE_LISTEN_START': {
      return Object.freeze({
        ...state,
        voice: Object.freeze({ ...state.voice, listening: true, text: null }),
      });
    }

    case 'VOICE_RESULT': {
      const { number, confidence } = action;
      if (number === null || confidence < 0.5) {
        return Object.freeze({
          ...state,
          voice: Object.freeze({
            ...state.voice,
            listening: false,
            retries: state.voice.retries + 1,
            text: Object.freeze({
              display: "Didn't catch that! Tap mic to try again.",
              speech: "I didn't catch that! Tap the microphone to try again.",
            }),
            lastResult: action,
          }),
        });
      }
      if (confidence < 0.8) {
        return Object.freeze({
          ...state,
          voice: Object.freeze({
            ...state.voice,
            listening: false,
            confirming: true,
            confirmNumber: number,
            text: Object.freeze({
              display: `Did you say ${number}?`,
              speech: `Did you say ${number}?`,
            }),
            lastResult: action,
          }),
        });
      }
      return Object.freeze({
        ...state,
        voice: Object.freeze({
          ...state.voice,
          listening: false,
          text: Object.freeze({
            display: `You said: ${number}!`,
            speech: `You said ${number}!`,
          }),
          lastResult: action,
        }),
      });
    }

    case 'VOICE_CONFIRM': {
      if (action.confirmed) {
        return Object.freeze({
          ...state,
          voice: Object.freeze({ ...state.voice, confirming: false }),
        });
      }
      return Object.freeze({
        ...state,
        voice: Object.freeze({
          ...state.voice,
          confirming: false,
          confirmNumber: null,
          retries: state.voice.retries + 1,
          text: Object.freeze({
            display: 'Okay! Tap mic to try again.',
            speech: 'Okay! Tap the microphone to try again.',
          }),
        }),
      });
    }

    case 'VOICE_ERROR': {
      const errorText = action.error === 'not-allowed'
        ? Object.freeze({ display: 'Mic blocked', speech: 'Microphone is blocked. Use the buttons instead.' })
        : Object.freeze({ display: "Didn't hear anything. Tap mic to try again!", speech: "I didn't hear anything. Tap the microphone to try again!" });
      return Object.freeze({
        ...state,
        voice: Object.freeze({ ...state.voice, listening: false, text: errorText }),
      });
    }

    default:
      return state;
  }
}
