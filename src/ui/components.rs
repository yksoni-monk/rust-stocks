/// UI components and utilities for the stock analysis application
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

/// Render a loading indicator
pub fn render_loading_indicator(f: &mut Frame, area: Rect, message: &str) {
    let loading = Paragraph::new(message)
        .block(Block::default().borders(Borders::ALL).title("Loading"))
        .style(Style::default().fg(Color::Yellow));
    
    f.render_widget(loading, area);
}

/// Render a progress bar
pub fn render_progress_bar(f: &mut Frame, area: Rect, progress: f64, label: &str) {
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(label))
        .gauge_style(Style::default().fg(Color::Green))
        .percent((progress * 100.0) as u16);
    
    f.render_widget(gauge, area);
}

/// Create a styled text span for positive/negative values
pub fn styled_change_span(value: f64, format_str: &str) -> Span<'static> {
    let formatted = if format_str.contains("{:.1}") {
        format!("{:.1}", value)
    } else if format_str.contains("{:.2}") {
        format!("{:.2}", value)
    } else {
        format!("{}", value)
    };
    if value >= 0.0 {
        Span::styled(formatted, Style::default().fg(Color::Green))
    } else {
        Span::styled(formatted, Style::default().fg(Color::Red))
    }
}

/// Create a percentage change span with + or - prefix
pub fn styled_percentage_change(value: f64) -> Span<'static> {
    let formatted = if value >= 0.0 {
        format!("+{:.1}%", value)
    } else {
        format!("{:.1}%", value)
    };
    
    if value >= 0.0 {
        Span::styled(formatted, Style::default().fg(Color::Green))
    } else {
        Span::styled(formatted, Style::default().fg(Color::Red))
    }
}

/// Format large numbers with commas
pub fn format_large_number(value: f64) -> String {
    if value >= 1_000_000_000_000.0 {
        format!("{:.1}T", value / 1_000_000_000_000.0)
    } else if value >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        format!("{:.0}", value)
    }
}

/// Create a simple ASCII chart from price data
pub fn create_ascii_chart(data: &[(f64, f64)], width: usize, height: usize) -> Vec<String> {
    if data.is_empty() || width == 0 || height == 0 {
        return vec!["No data".to_string()];
    }

    let min_val = data.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_val = data.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max);
    
    if (max_val - min_val).abs() < f64::EPSILON {
        return vec!["─".repeat(width); height];
    }

    let mut chart = vec![vec![' '; width]; height];
    
    for (i, (_, y)) in data.iter().enumerate() {
        if i >= width {
            break;
        }
        
        let normalized = (y - min_val) / (max_val - min_val);
        let row = ((1.0 - normalized) * (height - 1) as f64).round() as usize;
        let row = row.min(height - 1);
        
        chart[row][i] = '█';
    }
    
    chart.into_iter().map(|row| row.into_iter().collect()).collect()
}

/// Render error message
pub fn render_error(f: &mut Frame, area: Rect, error: &str) {
    let error_paragraph = Paragraph::new(error)
        .block(Block::default().borders(Borders::ALL).title("Error"))
        .style(Style::default().fg(Color::Red));
    
    f.render_widget(error_paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_large_number() {
        assert_eq!(format_large_number(1500.0), "1.5K");
        assert_eq!(format_large_number(1500000.0), "1.5M");
        assert_eq!(format_large_number(1500000000.0), "1.5B");
        assert_eq!(format_large_number(1500000000000.0), "1.5T");
        assert_eq!(format_large_number(500.0), "500");
    }

    #[test]
    fn test_create_ascii_chart() {
        let data = vec![(0.0, 10.0), (1.0, 20.0), (2.0, 15.0), (3.0, 25.0)];
        let chart = create_ascii_chart(&data, 4, 3);
        
        assert_eq!(chart.len(), 3);
        assert_eq!(chart[0].len(), 4);
        
        // Chart should show the progression of values
        assert!(chart.iter().any(|row| row.contains('█')));
    }

    #[test]
    fn test_styled_percentage_change() {
        // Test positive change
        let positive_span = styled_percentage_change(5.5);
        assert_eq!(positive_span.content, "+5.5%");
        
        // Test negative change
        let negative_span = styled_percentage_change(-3.2);
        assert_eq!(negative_span.content, "-3.2%");
        
        // Test zero change
        let zero_span = styled_percentage_change(0.0);
        assert_eq!(zero_span.content, "+0.0%");
    }
}