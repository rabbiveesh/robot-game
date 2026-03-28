// quiz-renderer.js — Multiple choice quiz renderer (the default/abstract CRA renderer)
// Reads from challengeState via window._challengeState

function createQuizRenderer() {
  return {
    render(ctx, cs, canvasW, canvasH, time) {
      if (!cs) return;

      ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
      ctx.fillRect(0, 0, canvasW, canvasH);

      if (cs.phase === 'teaching') {
        renderTeaching(ctx, canvasW, canvasH, time);
        return;
      }

      const panelW = Math.min(650, canvasW - 40);
      const panelH = cs.hintUsed ? 480 : 360;
      const panelX = (canvasW - panelW) / 2;
      const panelY = (canvasH - panelH) / 2 - 10;

      // Panel
      ctx.fillStyle = '#1a1a2e';
      roundRect(ctx, panelX, panelY, panelW, panelH, 16);
      ctx.fill();
      ctx.strokeStyle = '#FFD54F';
      ctx.lineWidth = 4;
      roundRect(ctx, panelX, panelY, panelW, panelH, 16);
      ctx.stroke();

      // Band badge
      const bandLabel = MATH_BAND_NAMES[SKILL.math.band] || '?';
      ctx.fillStyle = '#FFD54F';
      roundRect(ctx, panelX + panelW / 2 - 70, panelY - 16, 140, 32, 10);
      ctx.fill();
      ctx.fillStyle = '#1a1a2e';
      ctx.font = 'bold 16px "Segoe UI", system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(`# ${bandLabel}`, panelX + panelW / 2, panelY + 5);

      // Question
      ctx.fillStyle = '#FFF';
      ctx.font = 'bold 30px "Segoe UI", system-ui, sans-serif';
      ctx.textAlign = 'center';
      const qText = cs.question.display;
      const qLines = qText.split('\n');
      qLines.forEach((line, i) => {
        ctx.fillText(line, panelX + panelW / 2, panelY + 60 + i * 38);
      });

      // CRA visual hint (if show-me was used)
      let hintOffset = 0;
      if (cs.hintUsed && cs.challenge.numbers) {
        const td = cs.challenge.numbers;
        hintOffset = 70;
        const band = cs.challenge.sampledBand || cs.challenge.band || 1;
        // Use base-10 blocks for bands 5+, dots for bands 1-4
        if (band >= 5 && typeof renderBase10Blocks === 'function') {
          renderBase10Blocks(ctx, td.a, td.b, td.op, cs.challenge.correctAnswer,
            panelX + panelW / 2, panelY + 80 + qLines.length * 38, time);
        } else if (typeof renderDotVisual === 'function') {
          renderDotVisual(ctx, panelX + panelW / 2, panelY + 80 + qLines.length * 38,
            td.a, td.b, td.op, cs.challenge.correctAnswer, time);
        }
      }

      // Feedback (above buttons)
      let feedbackOffset = 0;
      if (cs.phase === 'feedback' && cs.feedback) {
        feedbackOffset = 35;
        ctx.font = 'bold 22px "Segoe UI", system-ui, sans-serif';
        ctx.fillStyle = '#FF8A65';
        ctx.textAlign = 'center';
        ctx.fillText(cs.feedback.display, panelX + panelW / 2,
          panelY + 120 + (qLines.length - 1) * 38 + hintOffset);
      }

      // Choice buttons
      const choices = cs.challenge.choices || [];
      const btnW = Math.min(160, (panelW - 80) / 3);
      const btnH = 70;
      const btnY = panelY + 130 + (qLines.length - 1) * 38 + hintOffset + feedbackOffset;
      const totalBtnW = btnW * 3 + 20 * 2;
      const btnStartX = panelX + (panelW - totalBtnW) / 2;

      choices.forEach((choice, i) => {
        const bx = btnStartX + i * (btnW + 20);
        const by = btnY;
        let btnColor = '#2196F3';
        if (cs.phase === 'complete' && cs.correct) {
          btnColor = choice.correct ? '#4CAF50' : '#37474F';
        }
        ctx.fillStyle = btnColor;
        roundRect(ctx, bx, by, btnW, btnH, 12);
        ctx.fill();
        ctx.strokeStyle = 'rgba(255,255,255,0.3)';
        ctx.lineWidth = 2;
        roundRect(ctx, bx, by, btnW, btnH, 12);
        ctx.stroke();
        ctx.fillStyle = '#FFF';
        ctx.font = 'bold 28px "Segoe UI", system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(choice.text, bx + btnW / 2, by + btnH / 2 + 10);
        choice._bounds = { x: bx, y: by, w: btnW, h: btnH };
      });

      // Show-me / Tell-me buttons (when not answered)
      if (cs.phase === 'presented' || cs.phase === 'feedback') {
        const scaffY = btnY + btnH + 12;
        const scaffBtnW = 90;
        const scaffBtnH = 30;
        const scaffGap = 10;

        // Show me (drops CRA one level)
        if (cs.renderHint.craStage !== 'concrete') {
          const smX = panelX + panelW / 2 - scaffBtnW - scaffGap / 2;
          ctx.fillStyle = '#546E7A';
          roundRect(ctx, smX, scaffY, scaffBtnW, scaffBtnH, 6);
          ctx.fill();
          ctx.fillStyle = '#B0BEC5';
          ctx.font = '13px "Segoe UI", system-ui, sans-serif';
          ctx.textAlign = 'center';
          ctx.fillText('Show me', smX + scaffBtnW / 2, scaffY + scaffBtnH / 2 + 4);
          window._showMeBounds = { x: smX, y: scaffY, w: scaffBtnW, h: scaffBtnH };
        } else {
          window._showMeBounds = null;
        }

        // Tell me (show the answer)
        const tmX = cs.renderHint.craStage !== 'concrete'
          ? panelX + panelW / 2 + scaffGap / 2
          : panelX + panelW / 2 - scaffBtnW / 2;
        ctx.fillStyle = '#455A64';
        roundRect(ctx, tmX, scaffY, scaffBtnW, scaffBtnH, 6);
        ctx.fill();
        ctx.fillStyle = '#90A4AE';
        ctx.font = '13px "Segoe UI", system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText('Tell me', tmX + scaffBtnW / 2, scaffY + scaffBtnH / 2 + 4);
        window._tellMeBounds = { x: tmX, y: scaffY, w: scaffBtnW, h: scaffBtnH };
      } else {
        window._showMeBounds = null;
        window._tellMeBounds = null;
      }

      // Celebration
      if (cs.phase === 'complete' && cs.correct) {
        ctx.font = 'bold 32px "Segoe UI", system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillStyle = '#FFD54F';
        const praises = ['AMAZING!', 'WOW!', 'GENIUS!', 'SO SMART!', 'INCREDIBLE!', 'YOU GOT IT!'];
        const praise = praises[Math.floor((time * 10) % praises.length)];
        ctx.fillText(praise, panelX + panelW / 2, btnY + btnH + 55);
        if (typeof drawStarBurst === 'function') {
          drawStarBurst(ctx, panelX + panelW / 2, btnY + btnH + 35, time, CHALLENGE.celebrationStart || time, 2);
        }
      }
    },

    handleClick(mx, my, cs) {
      if (!cs) return null;

      // Show-me button
      if (window._showMeBounds) {
        const b = window._showMeBounds;
        if (mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
          return { type: 'SHOW_ME' };
        }
      }

      // Tell-me button
      if (window._tellMeBounds) {
        const b = window._tellMeBounds;
        if (mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
          return { type: 'TELL_ME' };
        }
      }

      // Choice buttons
      const choices = cs.challenge.choices || [];
      for (let i = 0; i < choices.length; i++) {
        const b = choices[i]._bounds;
        if (b && mx >= b.x && mx <= b.x + b.w && my >= b.y && my <= b.y + b.h) {
          return { type: 'ANSWER_SUBMITTED', answer: Number(choices[i].text) };
        }
      }

      return null;
    },

    handleKey(key, cs) {
      if (!cs || cs.phase === 'complete' || cs.phase === 'teaching') return null;
      const num = parseInt(key);
      if (num >= 1 && num <= 3) {
        const choices = cs.challenge.choices || [];
        if (choices[num - 1]) {
          return { type: 'ANSWER_SUBMITTED', answer: Number(choices[num - 1].text) };
        }
      }
      return null;
    },

    dispose() {
      window._showMeBounds = null;
      window._tellMeBounds = null;
    },
  };
}

// Register with renderer registry
if (typeof registerRenderer === "function") {
  registerRenderer("quiz", {
    name: "Quiz (Multiple Choice)",
    description: "Standard multiple choice with Show-me/Tell-me buttons.",
  }, createQuizRenderer);
}
