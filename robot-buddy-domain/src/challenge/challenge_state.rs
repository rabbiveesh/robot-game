use serde::{Deserialize, Serialize};
use crate::types::{CraStage, Phase};
use crate::economy::rewards::{Reward, determine_reward};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderHint {
    pub cra_stage: CraStage,
    pub answer_mode: String,
    pub interaction_type: String,
}

impl Default for RenderHint {
    fn default() -> Self {
        RenderHint {
            cra_stage: CraStage::Abstract,
            answer_mode: "choice".into(),
            interaction_type: "quiz".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySpeech {
    pub display: String,
    pub speech: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceState {
    pub listening: bool,
    pub confirming: bool,
    pub confirm_number: Option<i32>,
    pub retries: u32,
    pub text: Option<DisplaySpeech>,
}

impl VoiceState {
    pub fn reset() -> Self {
        VoiceState { listening: false, confirming: false, confirm_number: None, retries: 0, text: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeState {
    pub phase: Phase,
    pub correct_answer: i32,
    pub attempts: u32,
    pub max_attempts: u32,
    pub correct: Option<bool>,
    pub question: DisplaySpeech,
    pub feedback: Option<DisplaySpeech>,
    pub reward: Option<Reward>,
    pub render_hint: RenderHint,
    pub hint_used: bool,
    pub hint_level: u32,
    pub told_me: bool,
    pub voice: VoiceState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ChallengeAction {
    AnswerSubmitted { answer: i32 },
    Retry,
    TeachingComplete,
    ShowMe,
    TellMe,
    VoiceListenStart,
    VoiceResult { number: Option<i32>, confidence: f64 },
    VoiceConfirm { confirmed: bool },
    VoiceError { error: String },
}

pub fn challenge_reducer(state: ChallengeState, action: ChallengeAction) -> ChallengeState {
    match action {
        ChallengeAction::AnswerSubmitted { answer } => {
            let correct = answer == state.correct_answer;
            let attempts = state.attempts + 1;

            if correct {
                ChallengeState {
                    phase: Phase::Complete,
                    correct: Some(true),
                    attempts,
                    reward: determine_reward(true),
                    feedback: Some(DisplaySpeech {
                        display: "Amazing! You got it!".into(),
                        speech: "Amazing! You got it!".into(),
                    }),
                    voice: VoiceState::reset(),
                    ..state
                }
            } else if attempts >= state.max_attempts {
                ChallengeState {
                    phase: Phase::Teaching,
                    correct: Some(false),
                    attempts,
                    reward: None,
                    feedback: Some(DisplaySpeech {
                        display: "Let's figure it out together!".into(),
                        speech: "Let's figure it out together!".into(),
                    }),
                    voice: VoiceState::reset(),
                    ..state
                }
            } else {
                ChallengeState {
                    phase: Phase::Feedback,
                    attempts,
                    feedback: Some(DisplaySpeech {
                        display: "Hmm, not quite! Try again!".into(),
                        speech: "Hmm, not quite! Try again!".into(),
                    }),
                    ..state
                }
            }
        }

        ChallengeAction::Retry => ChallengeState {
            phase: Phase::Presented,
            feedback: None,
            ..state
        },

        ChallengeAction::TeachingComplete => ChallengeState {
            phase: Phase::Complete,
            ..state
        },

        ChallengeAction::ShowMe => {
            let current = state.render_hint.cra_stage;
            let lower = match current {
                CraStage::Abstract => CraStage::Representational,
                CraStage::Representational => CraStage::Concrete,
                CraStage::Concrete => CraStage::Concrete,
            };
            ChallengeState {
                render_hint: RenderHint { cra_stage: lower, ..state.render_hint },
                hint_used: true,
                hint_level: state.hint_level + 1,
                ..state
            }
        }

        ChallengeAction::TellMe => ChallengeState {
            phase: Phase::Teaching,
            told_me: true,
            reward: None,
            feedback: Some(DisplaySpeech {
                display: format!("The answer is {}!", state.correct_answer),
                speech: format!("The answer is {}!", state.correct_answer),
            }),
            render_hint: RenderHint { cra_stage: CraStage::Concrete, ..state.render_hint },
            ..state
        },

        ChallengeAction::VoiceListenStart => ChallengeState {
            voice: VoiceState { listening: true, text: None, ..state.voice },
            ..state
        },

        ChallengeAction::VoiceResult { number, confidence } => {
            if number.is_none() || confidence < 0.5 {
                ChallengeState {
                    voice: VoiceState {
                        listening: false,
                        retries: state.voice.retries + 1,
                        text: Some(DisplaySpeech {
                            display: "Didn't catch that! Tap mic to try again.".into(),
                            speech: "I didn't catch that! Tap the microphone to try again.".into(),
                        }),
                        ..state.voice
                    },
                    ..state
                }
            } else if confidence < 0.8 {
                let n = number.unwrap();
                ChallengeState {
                    voice: VoiceState {
                        listening: false,
                        confirming: true,
                        confirm_number: Some(n),
                        text: Some(DisplaySpeech {
                            display: format!("Did you say {}?", n),
                            speech: format!("Did you say {}?", n),
                        }),
                        ..state.voice
                    },
                    ..state
                }
            } else {
                let n = number.unwrap();
                ChallengeState {
                    voice: VoiceState {
                        listening: false,
                        text: Some(DisplaySpeech {
                            display: format!("You said: {}!", n),
                            speech: format!("You said {}!", n),
                        }),
                        ..state.voice
                    },
                    ..state
                }
            }
        }

        ChallengeAction::VoiceConfirm { confirmed } => {
            if confirmed {
                ChallengeState {
                    voice: VoiceState { confirming: false, ..state.voice },
                    ..state
                }
            } else {
                ChallengeState {
                    voice: VoiceState {
                        confirming: false,
                        confirm_number: None,
                        retries: state.voice.retries + 1,
                        text: Some(DisplaySpeech {
                            display: "Okay! Tap mic to try again.".into(),
                            speech: "Okay! Tap the microphone to try again.".into(),
                        }),
                        ..state.voice
                    },
                    ..state
                }
            }
        }

        ChallengeAction::VoiceError { error } => {
            let text = if error == "not-allowed" {
                DisplaySpeech { display: "Mic blocked".into(), speech: "Microphone is blocked. Use the buttons instead.".into() }
            } else {
                DisplaySpeech { display: "Didn't hear anything. Tap mic to try again!".into(), speech: "I didn't hear anything. Tap the microphone to try again!".into() }
            };
            ChallengeState {
                voice: VoiceState { listening: false, text: Some(text), ..state.voice },
                ..state
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ChallengeState {
        ChallengeState {
            phase: Phase::Presented,
            correct_answer: 7,
            attempts: 0,
            max_attempts: 2,
            correct: None,
            question: DisplaySpeech { display: "What is 3 + 4?".into(), speech: "What is 3 plus 4?".into() },
            feedback: None,
            reward: None,
            render_hint: RenderHint::default(),
            hint_used: false,
            hint_level: 0,
            told_me: false,
            voice: VoiceState::reset(),
        }
    }

    #[test]
    fn correct_completes_with_reward() {
        let s = challenge_reducer(state(), ChallengeAction::AnswerSubmitted { answer: 7 });
        assert_eq!(s.phase, Phase::Complete);
        assert_eq!(s.correct, Some(true));
        assert!(s.reward.is_some());
    }

    #[test]
    fn wrong_goes_to_feedback() {
        let s = challenge_reducer(state(), ChallengeAction::AnswerSubmitted { answer: 5 });
        assert_eq!(s.phase, Phase::Feedback);
        assert!(s.reward.is_none());
    }

    #[test]
    fn two_wrongs_go_to_teaching() {
        let s = challenge_reducer(state(), ChallengeAction::AnswerSubmitted { answer: 5 });
        let s = challenge_reducer(s, ChallengeAction::AnswerSubmitted { answer: 3 });
        assert_eq!(s.phase, Phase::Teaching);
        assert_eq!(s.correct, Some(false));
    }

    #[test]
    fn show_me_at_concrete_still_sets_hint() {
        let mut s = state();
        s.render_hint.cra_stage = CraStage::Concrete;
        let s = challenge_reducer(s, ChallengeAction::ShowMe);
        assert!(s.hint_used);
        assert_eq!(s.render_hint.cra_stage, CraStage::Concrete);
    }

    #[test]
    fn tell_me_shows_answer() {
        let s = challenge_reducer(state(), ChallengeAction::TellMe);
        assert_eq!(s.phase, Phase::Teaching);
        assert!(s.told_me);
        assert!(s.feedback.as_ref().unwrap().display.contains("7"));
    }

    #[test]
    fn voice_low_confidence_retries() {
        let s = challenge_reducer(state(), ChallengeAction::VoiceResult { number: Some(7), confidence: 0.3 });
        assert_eq!(s.voice.retries, 1);
        assert_eq!(s.phase, Phase::Presented);
    }

    #[test]
    fn voice_medium_confidence_confirms() {
        let s = challenge_reducer(state(), ChallengeAction::VoiceResult { number: Some(7), confidence: 0.6 });
        assert!(s.voice.confirming);
        assert_eq!(s.voice.confirm_number, Some(7));
    }
}
