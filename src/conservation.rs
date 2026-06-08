use crate::types::{AgentBudget, FleetConservation};

/// Check that a single agent's budget satisfies the conservation invariant.
///
/// The invariant requires that `gamma + eta == total`.
/// Uses an epsilon of `1e-9` for floating-point tolerance.
pub fn check_conservation(budget: &AgentBudget) -> bool {
    (budget.gamma + budget.eta - budget.total).abs() < 1e-9
}

/// Compute fleet-wide conservation totals and check the global invariant.
///
/// Sums gamma, eta, and total across all agents, then checks that
/// `total_gamma + total_eta == total_budget`.
pub fn fleet_conservation(budgets: &[AgentBudget]) -> FleetConservation {
    let total_gamma: f64 = budgets.iter().map(|b| b.gamma).sum();
    let total_eta: f64 = budgets.iter().map(|b| b.eta).sum();
    let total_budget: f64 = budgets.iter().map(|b| b.total).sum();
    let invariant_holds = (total_gamma + total_eta - total_budget).abs() < 1e-9;

    FleetConservation {
        total_gamma,
        total_eta,
        total_budget,
        invariant_holds,
    }
}

/// Compute the deficit (or surplus) for a single agent budget.
///
/// Returns `total - (gamma + eta)`. Positive means under-allocated.
pub fn budget_deficit(budget: &AgentBudget) -> f64 {
    budget.total - (budget.gamma + budget.eta)
}

/// Validate all budgets and return the list of agent IDs that violate conservation.
pub fn violating_agents(budgets: &[AgentBudget]) -> Vec<String> {
    budgets
        .iter()
        .filter(|b| !check_conservation(b))
        .map(|b| b.agent_id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_budget(id: &str, gamma: f64, eta: f64, total: f64) -> AgentBudget {
        AgentBudget {
            agent_id: id.to_string(),
            gamma,
            eta,
            total,
        }
    }

    #[test]
    fn test_conservation_holds() {
        let b = make_budget("a1", 50.0, 50.0, 100.0);
        assert!(check_conservation(&b));
    }

    #[test]
    fn test_conservation_fails() {
        let b = make_budget("a1", 50.0, 40.0, 100.0);
        assert!(!check_conservation(&b));
    }

    #[test]
    fn test_conservation_zero() {
        let b = make_budget("a1", 0.0, 0.0, 0.0);
        assert!(check_conservation(&b));
    }

    #[test]
    fn test_conservation_floating_point() {
        // 0.1 + 0.2 ≈ 0.3 but not exactly in IEEE 754
        let b = make_budget("a1", 0.1, 0.2, 0.3);
        assert!(check_conservation(&b));
    }

    #[test]
    fn test_fleet_conservation_holds() {
        let budgets = vec![
            make_budget("a1", 30.0, 70.0, 100.0),
            make_budget("a2", 40.0, 60.0, 100.0),
        ];
        let fc = fleet_conservation(&budgets);
        assert!(fc.invariant_holds);
        assert_eq!(fc.total_gamma, 70.0);
        assert_eq!(fc.total_eta, 130.0);
        assert_eq!(fc.total_budget, 200.0);
    }

    #[test]
    fn test_fleet_conservation_fails() {
        let budgets = vec![
            make_budget("a1", 30.0, 70.0, 100.0),
            make_budget("a2", 40.0, 50.0, 100.0),
        ];
        let fc = fleet_conservation(&budgets);
        assert!(!fc.invariant_holds);
    }

    #[test]
    fn test_fleet_empty() {
        let fc = fleet_conservation(&[]);
        assert!(fc.invariant_holds);
        assert_eq!(fc.total_gamma, 0.0);
    }

    #[test]
    fn test_budget_deficit_positive() {
        let b = make_budget("a1", 30.0, 30.0, 100.0);
        assert_eq!(budget_deficit(&b), 40.0);
    }

    #[test]
    fn test_budget_deficit_zero() {
        let b = make_budget("a1", 50.0, 50.0, 100.0);
        assert_eq!(budget_deficit(&b), 0.0);
    }

    #[test]
    fn test_violating_agents_none() {
        let budgets = vec![
            make_budget("a1", 50.0, 50.0, 100.0),
            make_budget("a2", 40.0, 60.0, 100.0),
        ];
        assert!(violating_agents(&budgets).is_empty());
    }

    #[test]
    fn test_violating_agents_some() {
        let budgets = vec![
            make_budget("a1", 50.0, 50.0, 100.0),
            make_budget("a2", 40.0, 50.0, 100.0),
        ];
        let violators = violating_agents(&budgets);
        assert_eq!(violators, vec!["a2".to_string()]);
    }
}
