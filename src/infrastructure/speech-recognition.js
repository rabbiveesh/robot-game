// speech-recognition.js — Voice input for number answers
// Zero domain dependencies. Pure functions + thin browser API wrapper.

// ─── NUMBER PARSER ──────────────────────────────────────

const ONES = {
  zero: 0, one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7,
  eight: 8, nine: 9, ten: 10, eleven: 11, twelve: 12, thirteen: 13,
  fourteen: 14, fifteen: 15, sixteen: 16, seventeen: 17, eighteen: 18, nineteen: 19,
};

const TENS = {
  twenty: 20, thirty: 30, forty: 40, fifty: 50,
  sixty: 60, seventy: 70, eighty: 80, ninety: 90,
};

const FILLERS = ['um', 'uh', 'umm', 'erm', 'like', 'i think', "it's", 'its', 'is it', 'maybe', 'i say', 'i said'];

// Parse a single number phrase (no self-correction handling)
function parseSingleNumber(text) {
  const words = text.split(/[\s-]+/).filter(w => w.length > 0);
  if (words.length === 0) return null;

  // If the entire input is a single digit string, return it directly
  const pureDigit = text.trim().match(/^\d+$/);
  if (pureDigit) return parseInt(pureDigit[0]);

  let result = 0;
  let current = 0;
  let found = false;

  for (const word of words) {
    // Check for inline digit
    const digitMatch = word.match(/^\d+$/);
    if (digitMatch) {
      current += parseInt(digitMatch[0]);
      found = true;
    } else if (ONES[word] !== undefined) {
      current += ONES[word];
      found = true;
    } else if (TENS[word] !== undefined) {
      current += TENS[word];
      found = true;
    } else if (word === 'hundred') {
      if (current === 0) current = 1; // "a hundred"
      current *= 100;
      found = true;
    } else if (word === 'a' || word === 'and') {
      // skip connectors
    }
    // ignore unknown words
  }

  if (!found) return null;
  result += current;
  return result;
}

export function parseSpokenNumber(transcript) {
  if (!transcript || typeof transcript !== 'string') return null;

  let text = transcript.toLowerCase().trim();
  if (!text) return null;

  // Strip filler words/phrases
  for (const f of FILLERS) {
    text = text.replace(new RegExp(`\\b${f.replace(/['']/g, "[''']?")}\\b`, 'gi'), '');
  }
  text = text.replace(/[?.!,]/g, '').trim();
  if (!text) return null;

  // Split on correction words to handle self-correction ("twelve no thirteen")
  // Take the last segment that parses as a number
  const segments = text.split(/\b(?:no|wait|actually|i mean)\b/);
  for (let i = segments.length - 1; i >= 0; i--) {
    const seg = segments[i].trim();
    if (!seg) continue;
    const num = parseSingleNumber(seg);
    if (num !== null) return num;
  }

  // Last resort: try the whole cleaned string
  return parseSingleNumber(text);
}

// ─── BROWSER API ────────────────────────────────────────

export function isVoiceAvailable() {
  return !!(typeof window !== 'undefined' &&
    (window.SpeechRecognition || window.webkitSpeechRecognition));
}

let _recognition = null;

export function listenForNumber(options = {}) {
  const timeoutMs = options.timeoutMs || 10000;
  const lang = options.lang || 'en-US';

  if (!isVoiceAvailable()) {
    return Promise.reject(new Error('Speech recognition not available'));
  }

  // Stop any ongoing recognition
  stopListening();

  const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
  _recognition = new SpeechRecognition();
  _recognition.lang = lang;
  _recognition.interimResults = true;
  _recognition.maxAlternatives = 3;
  _recognition.continuous = false;

  return new Promise((resolve, reject) => {
    const startTime = performance.now();
    let firstSpeechTime = null;
    let interimHistory = [];
    let hadFillerWords = false;
    let timeoutId = null;

    timeoutId = setTimeout(() => {
      stopListening();
      reject(new Error('timeout'));
    }, timeoutMs);

    _recognition.onresult = (event) => {
      const result = event.results[event.results.length - 1];

      // Track interim results for self-correction detection
      if (!result.isFinal) {
        const interim = result[0].transcript;
        interimHistory.push(interim);

        // Track first speech time (hesitation)
        if (firstSpeechTime === null && interim.trim()) {
          firstSpeechTime = performance.now();
        }
        return;
      }

      // Final result
      clearTimeout(timeoutId);
      const totalMs = performance.now() - startTime;
      const hesitationMs = firstSpeechTime ? firstSpeechTime - startTime : totalMs;

      const transcript = result[0].transcript;
      const confidence = result[0].confidence;

      // Check for filler words in the raw transcript
      const lowerTranscript = transcript.toLowerCase();
      hadFillerWords = FILLERS.some(f => lowerTranscript.includes(f));

      // Detect self-correction: did interim results show a different number?
      let selfCorrected = false;
      if (interimHistory.length >= 2) {
        const earlyNum = parseSpokenNumber(interimHistory[0]);
        const finalNum = parseSpokenNumber(transcript);
        if (earlyNum !== null && finalNum !== null && earlyNum !== finalNum) {
          selfCorrected = true;
        }
      }

      // Parse alternatives
      const alternatives = [];
      for (let i = 1; i < result.length; i++) {
        alternatives.push({
          transcript: result[i].transcript,
          number: parseSpokenNumber(result[i].transcript),
          confidence: result[i].confidence,
        });
      }

      resolve({
        transcript,
        number: parseSpokenNumber(transcript),
        confidence,
        alternatives,
        hesitationMs: Math.round(hesitationMs),
        totalMs: Math.round(totalMs),
        selfCorrected,
        hadFillerWords: hadFillerWords,
        raw: event.results,
      });
    };

    _recognition.onerror = (event) => {
      clearTimeout(timeoutId);
      if (event.error === 'no-speech') {
        reject(new Error('no-speech'));
      } else if (event.error === 'not-allowed') {
        reject(new Error('not-allowed'));
      } else {
        reject(new Error(event.error || 'unknown'));
      }
    };

    _recognition.onend = () => {
      // If we get here without a result, it timed out or had no speech
    };

    _recognition.start();
  });
}

export function stopListening() {
  if (_recognition) {
    try { _recognition.abort(); } catch (e) { /* ignore */ }
    _recognition = null;
  }
}
