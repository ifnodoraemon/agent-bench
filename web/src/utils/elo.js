/**
 * Pairwise Elo Calculator for Benchmark Models
 */
export function computeHeadToHeadElo(summaries, kFactor = 24.0, initialRating = 1200.0) {
  if (!summaries || summaries.length < 2) return summaries;

  const ratings = {};
  summaries.forEach(s => {
    ratings[s.model_id] = initialRating;
  });

  const battles = [];
  const n = summaries.length;

  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      const mA = summaries[i];
      const mB = summaries[j];

      // Build case lookup map for mB
      const mapB = new Map();
      (mB.case_results || []).forEach(c => mapB.set(c.test_case_id, c));

      (mA.case_results || []).forEach(caseA => {
        const caseB = mapB.get(caseA.test_case_id);
        if (caseB) {
          let scoreA = 0.5;
          if (caseA.score > caseB.score) {
            scoreA = 1.0;
          } else if (caseA.score < caseB.score) {
            scoreA = 0.0;
          }
          battles.push({ idA: mA.model_id, idB: mB.model_id, scoreA });
        }
      });
    }
  }

  // Iterate Elo updates
  battles.forEach(({ idA, idB, scoreA }) => {
    const rA = ratings[idA];
    const rB = ratings[idB];
    const eA = 1.0 / (1.0 + Math.pow(10, (rB - rA) / 400.0));
    const eB = 1.0 - eA;

    ratings[idA] += kFactor * (scoreA - eA);
    ratings[idB] += kFactor * ((1.0 - scoreA) - eB);
  });

  // Assign updated Elo back to summaries
  return summaries.map(s => ({
    ...s,
    computed_elo: Math.round(ratings[s.model_id] || initialRating)
  }));
}

/**
 * Head-to-head comparison between two models
 */
export function compareTwoModels(modelA, modelB) {
  if (!modelA || !modelB) return null;

  const casesA = modelA.case_results || [];
  const casesB = modelB.case_results || [];

  const mapB = new Map();
  casesB.forEach(c => mapB.set(c.test_case_id, c));

  let winsA = 0;
  let winsB = 0;
  let ties = 0;
  const commonCases = [];

  casesA.forEach(cA => {
    const cB = mapB.get(cA.test_case_id);
    if (cB) {
      let outcome = 'tie';
      if (cA.score > cB.score) {
        outcome = 'winA';
        winsA++;
      } else if (cA.score < cB.score) {
        outcome = 'winB';
        winsB++;
      } else {
        ties++;
      }
      commonCases.push({
        id: cA.test_case_id,
        category: cA.category,
        difficulty: cA.difficulty || 'L3',
        caseA: cA,
        caseB: cB,
        outcome
      });
    }
  });

  const total = winsA + winsB + ties;
  return {
    modelA,
    modelB,
    totalCompared: total,
    winsA,
    winsB,
    ties,
    winRateA: total > 0 ? (winsA / total) * 100 : 0,
    winRateB: total > 0 ? (winsB / total) * 100 : 0,
    tieRate: total > 0 ? (ties / total) * 100 : 0,
    cases: commonCases
  };
}
