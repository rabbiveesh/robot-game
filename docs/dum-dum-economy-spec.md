# Dum Dum Economy — Design Spec (DRAFT)

## The Problem

Dum Dums are currently a number that goes up. Kids expect to actually give them to Sparky. The counter is a scoreboard, not an economy. There's no spending, no choices, no math embedded in the resource management.

## Design Goals

1. **Giving Dum Dums to Sparky is a first-class interaction.** Walk up to Sparky, press a button, hand one over. Sparky reacts with joy. This is the emotional core — relatedness, not math.

2. **Spending is embedded math.** Every purchase is a subtraction problem. Every saving goal is a "how many more do I need?" problem. The economy IS math practice without feeling like it.

3. **Spending behavior is signal.** How a kid manages resources reveals cognitive traits the adaptive system can use.

## Interactions

### Give to Sparky (always available)

When facing Sparky, a "Give Dum Dum" option appears alongside "Talk." Costs 1 Dum Dum.

**Sparky's reactions** (random, escalating with total gifts given):
- First few: "MMMMM! *crunch* BEST BOSS EVER! My circuits are tingling!"
- After 5: Sparky does a little spin animation
- After 10: Sparky's antenna changes color temporarily
- After 20: Sparky gets a visible accessory (bow tie? hat? changes per milestone)
- After 50: Sparky's chest light changes color permanently. "Boss, you've given me FIFTY Dum Dums. I'm the luckiest robot in the WORLD."

These are cosmetic rewards. The kid is buying love, basically. And that's fine — it's a healthy emotional dynamic. Sparky is grateful and expressive.

### Shop purchases

The shopkeeper (Bolt) sells cosmetic items for Sparky:
- Hat: 3 Dum Dums
- Bow tie: 5 Dum Dums
- Jet boots (visual only): 8 Dum Dums
- Color change: 10 Dum Dums
- Sparkle trail: 15 Dum Dums

**Each purchase is a math moment:**
- "That costs 5 Dum Dums. You have 12. How many will you have left?" → embedded subtraction
- "You need 15 but only have 9. How many more do you need?" → embedded number bonds
- "Buy 2 hats and a bow tie. How much total?" → embedded multi-step

These can use the full interaction model (CRA visuals, show-me, answer mode) or be presented as simple dialogue choices. The adaptive system determines which based on the kid's profile.

### Trading with NPCs

Other NPCs occasionally want Dum Dums in exchange for things:
- Professor Gizmo: "I need 4 Dum Dums for my experiment! I'll give you a crystal."
- Mommy: "Can you share your Dum Dums with the 3 friends at the park? How many each?" → division

### Earning

Dum Dums come from:
- Completing challenges correctly (existing)
- Treasure chests (existing)
- Quest rewards
- Finding hidden ones on the map (exploration reward)
- Mini-quests from NPCs

## Signal Extraction

### Spending Events

```js
DumDumSpent {
  type: 'DUM_DUM_SPENT',
  amount: number,
  recipient: 'sparky' | 'shop' | 'npc',
  item: string | null,        // what they bought
  balance_before: number,
  balance_after: number,
  timestamp: number,
}

DumDumEarned {
  type: 'DUM_DUM_EARNED',
  amount: number,
  source: 'challenge' | 'chest' | 'quest' | 'found',
  timestamp: number,
}
```

### Behavioral Patterns

| Pattern | What it might indicate | How the adaptive system could use it |
|---------|----------------------|--------------------------------------|
| Gives to Sparky frequently | High relatedness need. Emotionally engaged. | More Sparky dialogue, more companion interactions. |
| Hoards, never spends | Risk-averse or saving for something specific. | System can offer low-cost items to encourage first purchase. |
| Spends immediately on earning | Impulsive, wants instant gratification. | Might benefit from challenges that reward delayed gratification (save for X). |
| Saves for expensive items | Can plan ahead, comfortable with delayed reward. | May be ready for multi-step problems and longer quests. |
| Asks "how many more do I need?" (via show-me in shop) | Actively doing math to plan purchases. | Economy is working as a math practice layer. |
| Gives all Dum Dums away, always at 0 | Generous to a fault, or doesn't value the resource. | Might need earning to feel more meaningful. |

### Integration with Learner Profile

We don't need a dedicated "economy dial." The spending events feed into existing signals:
- Shop purchases where the kid does the subtraction correctly → same as a challenge attempt
- Planning behavior (saving, comparing prices) → evidence of multi-step reasoning
- Impulsive spending → may correlate with rapid-clicking, inform frustration detection

## Implementation Priority

**Phase 1 (cheap, high impact):**
- "Give Dum Dum" button when facing Sparky
- Sparky reactions (dialogue + simple animation)
- Track DumDumSpent/DumDumEarned events in the event log

**Phase 2 (medium):**
- Shop items that change Sparky's appearance
- Purchase math embedded in shop dialogue (using interaction model)

**Phase 3 (future):**
- NPC trading
- Multi-step purchase decisions ("buy 2 of these and 1 of that")
- Saving goals ("Sparky wants jet boots! 8 more Dum Dums to go!")
- Price comparison ("Shop A vs Shop B")

## Open Questions

- Should there be a "Dum Dum bank" where the kid can see their total and their spending history? (Math practice: "You started with 20, spent 7, earned 3. How many now?")
- Should some items be limited quantity? ("Only 2 sparkle trails left!") Creates urgency and comparison shopping.
- Should Sparky's mood/energy be visually affected by Dum Dum gifts? (More gifts = happier/bouncier Sparky? Could be a visible feedback loop that motivates giving.)
- Can Dum Dums be shared between save files? (Sibling economy? "Your brother saved 30 Dum Dums for you." Probably too complex for now.)

## Presentation Migration

**See `docs/presentation-migration.md` for migration trigger and plan.**
