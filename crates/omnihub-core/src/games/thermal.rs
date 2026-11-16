//! How hot the PC ran during a game: the processor's and graphics card's
//! temperatures every few seconds while a boost runs, summed up when the
//! game closes, with a plain verdict. Past these points parts slow
//! themselves down to cool (thermal throttling), which shows as lower FPS.

use serde::{Deserialize, Serialize};

/// Where most processors start holding back (some AMD chips run at 95 °C
/// by design, so this is "hot", not "broken").
pub const CPU_HOT: f32 = 90.0;
/// Where most graphics cards start lowering their clocks.
pub const GPU_HOT: f32 = 83.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThermalReport {
    pub max_cpu: Option<f32>,
    pub max_gpu: Option<f32>,
    pub avg_cpu: Option<f32>,
    pub avg_gpu: Option<f32>,
    /// Share of the game (0–1) spent at or over the hot point.
    pub hot_cpu: f32,
    pub hot_gpu: f32,
    pub verdict: String,
}

#[derive(Debug, Default)]
pub struct ThermalLog {
    cpu: Vec<f32>,
    gpu: Vec<f32>,
}

fn stats(v: &[f32], hot: f32) -> (Option<f32>, Option<f32>, f32) {
    if v.is_empty() {
        return (None, None, 0.0);
    }
    let max = v.iter().copied().fold(f32::MIN, f32::max);
    let avg = v.iter().sum::<f32>() / v.len() as f32;
    let share = v.iter().filter(|t| **t >= hot).count() as f32 / v.len() as f32;
    (Some(max), Some(avg), share)
}

impl ThermalLog {
    pub fn clear(&mut self) {
        self.cpu.clear();
        self.gpu.clear();
    }

    pub fn add(&mut self, cpu: Option<f32>, gpu: Option<f32>) {
        if let Some(t) = cpu.filter(|t| (1.0..130.0).contains(t)) {
            self.cpu.push(t);
        }
        if let Some(t) = gpu.filter(|t| (1.0..130.0).contains(t)) {
            self.gpu.push(t);
        }
    }

    /// None until there are a few readings.
    pub fn report(&self) -> Option<ThermalReport> {
        if self.cpu.len() < 3 && self.gpu.len() < 3 {
            return None;
        }
        let (max_cpu, avg_cpu, hot_cpu) = stats(&self.cpu, CPU_HOT);
        let (max_gpu, avg_gpu, hot_gpu) = stats(&self.gpu, GPU_HOT);
        let mut hot = Vec::new();
        if hot_cpu >= 0.1 {
            hot.push(format!("the processor reached {:.0} °C ({:.0}% of the game)", max_cpu.unwrap_or(0.0), hot_cpu * 100.0));
        }
        if hot_gpu >= 0.1 {
            hot.push(format!("the graphics card reached {:.0} °C ({:.0}% of the game)", max_gpu.unwrap_or(0.0), hot_gpu * 100.0));
        }
        let verdict = if hot.is_empty() {
            "Cool enough: nothing got hot enough to slow down.".to_string()
        } else {
            format!("Likely slowing down to cool: {}. Clean dust filters, raise fan speeds (My PC → Fans) or give the case more air.", hot.join(" and "))
        };
        Some(ThermalReport { max_cpu, max_gpu, avg_cpu, avg_gpu, hot_cpu, hot_gpu, verdict })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_heat() {
        let mut l = ThermalLog::default();
        assert!(l.report().is_none());
        for _ in 0..10 {
            l.add(Some(70.0), Some(65.0));
        }
        l.add(None, Some(200.0));
        let r = l.report().unwrap();
        assert_eq!((r.max_cpu, r.max_gpu, r.hot_cpu), (Some(70.0), Some(65.0), 0.0));
        assert!(r.verdict.starts_with("Cool enough"));
        for _ in 0..10 {
            l.add(Some(72.0), Some(86.0));
        }
        let r = l.report().unwrap();
        assert!((r.hot_gpu - 0.5).abs() < 0.01);
        assert!(r.verdict.contains("graphics card reached 86 °C (50% of the game)"), "{}", r.verdict);
        l.clear();
        assert!(l.report().is_none());
    }
}
