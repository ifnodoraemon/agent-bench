use std::collections::HashMap;

pub struct EloCalculator {
    pub k_factor: f64,
    pub initial_rating: f64,
}

impl Default for EloCalculator {
    fn default() -> Self {
        Self {
            k_factor: 32.0,
            initial_rating: 1000.0,
        }
    }
}

impl EloCalculator {
    pub fn new(k_factor: f64, initial_rating: f64) -> Self {
        Self {
            k_factor,
            initial_rating,
        }
    }

    /// Calculate expected score of A against B
    pub fn expected_score(rating_a: f64, rating_b: f64) -> f64 {
        1.0 / (1.0 + 10.0_f64.powf((rating_b - rating_a) / 400.0))
    }

    /// Update ratings after a match
    /// score_a: 1.0 for win, 0.5 for tie, 0.0 for loss
    pub fn update_rating(&self, rating_a: f64, rating_b: f64, score_a: f64) -> (f64, f64) {
        let expected_a = Self::expected_score(rating_a, rating_b);
        let expected_b = 1.0 - expected_a;

        let new_a = rating_a + self.k_factor * (score_a - expected_a);
        let new_b = rating_b + self.k_factor * ((1.0 - score_a) - expected_b);

        (new_a, new_b)
    }

    /// Compute Elo ratings across a series of battles
    pub fn compute_ratings(
        &self,
        models: &[String],
        battles: &[(&str, &str, f64)], // (model_a, model_b, score_a)
    ) -> HashMap<String, f64> {
        let mut ratings: HashMap<String, f64> = models
            .iter()
            .map(|m| (m.clone(), self.initial_rating))
            .collect();

        for &(model_a, model_b, score_a) in battles {
            let r_a = ratings.get(model_a).cloned().unwrap_or(self.initial_rating);
            let r_b = ratings.get(model_b).cloned().unwrap_or(self.initial_rating);
            let (new_a, new_b) = self.update_rating(r_a, r_b, score_a);
            ratings.insert(model_a.to_string(), new_a);
            ratings.insert(model_b.to_string(), new_b);
        }

        ratings
    }
}
