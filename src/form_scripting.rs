use crate::models::{FormField, FormFieldVariant};
use std::collections::HashMap;

/// Represents an Acrobat Form Field value inside the scripting runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum FormFieldValue {
    Number(f64),
    Text(String),
    Boolean(bool),
    Null,
}

impl FormFieldValue {
    pub fn as_f64(&self) -> f64 {
        match self {
            FormFieldValue::Number(n) => *n,
            FormFieldValue::Text(s) => s.trim().parse::<f64>().unwrap_or(0.0),
            FormFieldValue::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            FormFieldValue::Null => 0.0,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            FormFieldValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e12 {
                    format!("{:.0}", n)
                } else {
                    format!("{:.2}", n)
                }
            }
            FormFieldValue::Text(s) => s.clone(),
            FormFieldValue::Boolean(b) => {
                if *b {
                    "Yes".to_string()
                } else {
                    "Off".to_string()
                }
            }
            FormFieldValue::Null => String::new(),
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            FormFieldValue::Number(n) => *n != 0.0 && !n.is_nan(),
            FormFieldValue::Text(s) => !s.trim().is_empty() && s != "0" && s != "Off",
            FormFieldValue::Boolean(b) => *b,
            FormFieldValue::Null => false,
        }
    }
}

/// Acrobat JavaScript event state passed into calculation and validation hooks.
#[derive(Debug, Clone)]
pub struct FormEvent {
    pub target: String,
    pub value: FormFieldValue,
    pub rc: bool, // Return code (for validation: true = valid, false = rejected)
}

/// Core execution engine for Acrobat-compatible JavaScript form calculations.
#[derive(Debug, Clone)]
pub struct PdfFormScriptEngine {
    fields: HashMap<String, FormFieldValue>,
}

impl PdfFormScriptEngine {
    /// Initializes the script engine from the document's extracted `FormField` list.
    ///
    /// Text and Combo fields keep their exact string. Coercing a numeric-looking
    /// value to `Number` at construction discarded the representation, so `"007"`
    /// came back as `"7"` and `"3.50"` as `"3.5"` the moment the value was read.
    /// Only a calculation produces a number.
    pub fn new(form_fields: &[FormField]) -> Self {
        let mut fields = HashMap::new();

        for f in form_fields {
            let val = match &f.variant {
                FormFieldVariant::Text { value } => FormFieldValue::Text(value.clone()),
                FormFieldVariant::Checkbox { is_checked, .. } => {
                    FormFieldValue::Boolean(*is_checked)
                }
                FormFieldVariant::RadioButton { is_selected, .. } => {
                    FormFieldValue::Boolean(*is_selected)
                }
                FormFieldVariant::ComboBox {
                    selected_index,
                    export_values,
                    options,
                } => {
                    let text = selected_index
                        .and_then(|idx| export_values.get(idx).or_else(|| options.get(idx)))
                        .cloned()
                        .unwrap_or_default();
                    FormFieldValue::Text(text)
                }
            };
            fields.insert(f.name.clone(), val);
        }

        Self { fields }
    }

    /// Sets or updates a single field's value in the runtime environment.
    pub fn set_field_value(&mut self, name: &str, value: FormFieldValue) {
        self.fields.insert(name.to_string(), value);
    }

    /// Retrieves a field's value in the runtime environment.
    pub fn get_field_value(&self, name: &str) -> FormFieldValue {
        self.fields
            .get(name)
            .cloned()
            .unwrap_or(FormFieldValue::Null)
    }

    /// Evaluates an Acrobat JavaScript calculation script for `target_field`.
    ///
    /// Handles:
    /// - Standard Acrobat built-ins: `AFSimple_Calculate("SUM", ...)`
    /// - `this.getField("name").value` expressions
    /// - Direct arithmetic formulas: `A * B + C`
    /// - `event.value = ...` assignments
    pub fn evaluate_calculation(
        &mut self,
        target_field: &str,
        script: &str,
    ) -> Result<String, String> {
        let clean_script = script.trim();
        if clean_script.is_empty() {
            return Ok(self.get_field_value(target_field).as_string());
        }

        let mut event = FormEvent {
            target: target_field.to_string(),
            value: self.get_field_value(target_field),
            rc: true,
        };

        // 1. Check for standard Acrobat calculation function:
        // AFSimple_Calculate(cFunction, cFields)
        match self.eval_af_simple_calculate(clean_script) {
            Ok(Some(result)) => {
                event.value = FormFieldValue::Number(result);
                let out_str = event.value.as_string();
                self.set_field_value(target_field, event.value);
                return Ok(out_str);
            }
            Ok(None) => {}
            // An `AFSimple_Calculate` call whose operation we do not implement is
            // reported, not evaluated to zero.
            Err(e) => return Err(e),
        }

        // 2. Parse general calculation expressions:
        // e.g. `event.value = this.getField("Subtotal").value * 0.18;`
        let expr = if let Some(idx) = clean_script.find("event.value") {
            let after_target = &clean_script[idx + "event.value".len()..];
            if let Some(eq_idx) = after_target.find('=') {
                after_target[eq_idx + 1..]
                    .trim()
                    .trim_end_matches(';')
                    .trim()
            } else {
                clean_script
            }
        } else {
            clean_script
        };

        let calculated_val = self.eval_expression(expr)?;
        event.value = calculated_val;
        let out_str = event.value.as_string();
        self.set_field_value(target_field, event.value);
        Ok(out_str)
    }

    /// Evaluates an Acrobat validation script (checks whether `event.rc == true`).
    pub fn evaluate_validation(
        &mut self,
        target_field: &str,
        candidate_value: &str,
        script: &str,
    ) -> Result<bool, String> {
        let clean_script = script.trim();
        if clean_script.is_empty() {
            return Ok(true);
        }

        let val = if let Ok(n) = candidate_value.trim().parse::<f64>() {
            FormFieldValue::Number(n)
        } else {
            FormFieldValue::Text(candidate_value.to_string())
        };

        let mut event = FormEvent {
            target: target_field.to_string(),
            value: val,
            rc: true,
        };

        // Handle simple range checks or expressions
        // e.g., `event.rc = (event.value >= 0 && event.value <= 100);`
        if let Some(idx) = clean_script.find("event.rc") {
            let after_target = &clean_script[idx + "event.rc".len()..];
            if let Some(eq_idx) = after_target.find('=') {
                let expr = after_target[eq_idx + 1..]
                    .trim()
                    .trim_end_matches(';')
                    .trim();
                let res = self.eval_boolean_expression(expr, &event)?;
                event.rc = res;
            }
        }

        Ok(event.rc)
    }

    /// Evaluates Acrobat's standard `AFSimple_Calculate(cFunction, cFields)` helper.
    ///
    /// Operations:
    /// - `"SUM"`: Sum of all listed fields
    /// - `"PRD"`: Product of all listed fields
    /// - `"AVG"`: Average of all listed fields
    /// - `"MIN"`: Minimum of all listed fields
    /// - `"MAX"`: Maximum of all listed fields
    ///
    /// Returns `Ok(None)` when the script is not an `AFSimple_Calculate` call at
    /// all (so the caller can try the expression path), and `Err` for a call
    /// whose operation is unknown. An unknown operation used to evaluate to
    /// `0.0`, which is a perfectly plausible sum and therefore silent.
    fn eval_af_simple_calculate(&self, script: &str) -> Result<Option<f64>, String> {
        if !script.contains("AFSimple_Calculate") {
            return Ok(None);
        }

        let start_paren = script
            .find('(')
            .ok_or_else(|| "unterminated AFSimple_Calculate call (missing '(')".to_string())?;
        let end_paren = script
            .rfind(')')
            .ok_or_else(|| "unterminated AFSimple_Calculate call (missing ')')".to_string())?;
        let args_str = &script[start_paren + 1..end_paren];

        // Parse operation string (e.g., "SUM" or 'PRD')
        let mut parts = args_str.splitn(2, ',');
        let op_part = parts
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches(|c| c == '"' || c == '\'');
        let fields_part = parts.next().unwrap_or("").trim();

        // Extract field names from Array/list: `["A", "B"]` or `new Array("A", "B")`
        let mut field_names = Vec::new();
        let array_content = if let Some(bracket_start) = fields_part.find('[') {
            let bracket_end = fields_part.rfind(']').ok_or_else(|| {
                "unterminated field list in AFSimple_Calculate (missing ']')".to_string()
            })?;
            &fields_part[bracket_start + 1..bracket_end]
        } else if let Some(arr_start) = fields_part.find("Array(") {
            let arr_end = fields_part
                .rfind(')')
                .ok_or_else(|| "unterminated Array(...) in AFSimple_Calculate".to_string())?;
            &fields_part[arr_start + 6..arr_end]
        } else {
            fields_part
        };

        for item in array_content.split(',') {
            let clean_name = item
                .trim()
                .trim_matches(|c| c == '"' || c == '\'' || c == ' ');
            if !clean_name.is_empty() {
                field_names.push(clean_name);
            }
        }

        if field_names.is_empty() {
            return Ok(Some(0.0));
        }

        let nums: Vec<f64> = field_names
            .iter()
            .map(|name| self.get_field_value(name).as_f64())
            .collect();

        let result = match op_part.to_uppercase().as_str() {
            "SUM" => nums.iter().sum::<f64>(),
            "PRD" => nums.iter().product::<f64>(),
            "AVG" => nums.iter().sum::<f64>() / nums.len() as f64,
            "MIN" => nums.into_iter().fold(f64::INFINITY, f64::min),
            "MAX" => nums.into_iter().fold(f64::NEG_INFINITY, f64::max),
            other => {
                return Err(format!(
                    "unsupported AFSimple_Calculate operation `{other}` \
                     (expected SUM, PRD, AVG, MIN or MAX)"
                ));
            }
        };

        Ok(Some(result))
    }

    /// Evaluates an arithmetic expression with `this.getField("...").value` substitutions.
    fn eval_expression(&self, raw_expr: &str) -> Result<FormFieldValue, String> {
        let mut expr = raw_expr.to_string();

        // 1. Substitute `this.getField("Field").value` or `getField("Field").value`
        while let Some(pos) = expr.find("getField(") {
            let after = &expr[pos + "getField(".len()..];
            let end_quote = after
                .find(')')
                .ok_or_else(|| "Unmatched parenthesis in getField call".to_string())?;
            let raw_arg = &after[..end_quote].trim();
            let field_name = raw_arg.trim_matches(|c| c == '"' || c == '\'');

            let field_val = self.get_field_value(field_name).as_f64();

            // Find full pattern including potential `.value`
            let full_match_end = if after[end_quote..].starts_with(").value") {
                pos + "getField(".len() + end_quote + 7
            } else {
                pos + "getField(".len() + end_quote + 1
            };

            let full_match_start = if pos >= 5 && &expr[pos - 5..pos] == "this." {
                pos - 5
            } else {
                pos
            };

            expr.replace_range(full_match_start..full_match_end, &field_val.to_string());
        }

        // 2. Compute numeric formula
        let val = self.eval_simple_math(&expr)?;
        Ok(FormFieldValue::Number(val))
    }

    /// Evaluates boolean condition expressions for validation.
    fn eval_boolean_expression(&self, expr: &str, event: &FormEvent) -> Result<bool, String> {
        let clean = expr.replace("event.value", &event.value.as_f64().to_string());

        if clean.contains("&&") {
            let sub_exprs: Vec<&str> = clean.split("&&").collect();
            for sub in sub_exprs {
                if !self.eval_single_comparison(sub.trim())? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }

        if clean.contains("||") {
            let sub_exprs: Vec<&str> = clean.split("||").collect();
            for sub in sub_exprs {
                if self.eval_single_comparison(sub.trim())? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }

        self.eval_single_comparison(&clean)
    }

    fn eval_single_comparison(&self, comp: &str) -> Result<bool, String> {
        let clean = comp.trim().trim_matches(|c| c == '(' || c == ')');

        if let Some((left, right)) = clean.split_once(">=") {
            return Ok(self.eval_simple_math(left)? >= self.eval_simple_math(right)?);
        }
        if let Some((left, right)) = clean.split_once("<=") {
            return Ok(self.eval_simple_math(left)? <= self.eval_simple_math(right)?);
        }
        if let Some((left, right)) = clean.split_once("==") {
            return Ok((self.eval_simple_math(left)? - self.eval_simple_math(right)?).abs() < 1e-9);
        }
        if let Some((left, right)) = clean.split_once('>') {
            return Ok(self.eval_simple_math(left)? > self.eval_simple_math(right)?);
        }
        if let Some((left, right)) = clean.split_once('<') {
            return Ok(self.eval_simple_math(left)? < self.eval_simple_math(right)?);
        }

        // Truthy check fallback
        let val = self.eval_simple_math(clean)?;
        Ok(val != 0.0)
    }

    /// Evaluates simple recursive math expression (+, -, *, /)
    fn eval_simple_math(&self, expr: &str) -> Result<f64, String> {
        let clean = expr.trim();
        if clean.is_empty() {
            return Ok(0.0);
        }

        // Handle addition/subtraction at lowest precedence
        let mut depth = 0;
        for (i, c) in clean.char_indices().rev() {
            match c {
                ')' => depth += 1,
                '(' => depth -= 1,
                '+' if depth == 0 && i > 0 => {
                    let left = self.eval_simple_math(&clean[..i])?;
                    let right = self.eval_simple_math(&clean[i + 1..])?;
                    return Ok(left + right);
                }
                '-' if depth == 0 && i > 0 => {
                    let left = self.eval_simple_math(&clean[..i])?;
                    let right = self.eval_simple_math(&clean[i + 1..])?;
                    return Ok(left - right);
                }
                _ => {}
            }
        }

        // Handle multiplication/division
        depth = 0;
        for (i, c) in clean.char_indices().rev() {
            match c {
                ')' => depth += 1,
                '(' => depth -= 1,
                '*' if depth == 0 => {
                    let left = self.eval_simple_math(&clean[..i])?;
                    let right = self.eval_simple_math(&clean[i + 1..])?;
                    return Ok(left * right);
                }
                '/' if depth == 0 => {
                    let left = self.eval_simple_math(&clean[..i])?;
                    let right = self.eval_simple_math(&clean[i + 1..])?;
                    if right.abs() < 1e-12 {
                        return Ok(0.0); // Safe division by zero
                    }
                    return Ok(left / right);
                }
                _ => {}
            }
        }

        // Strip parentheses
        if clean.starts_with('(') && clean.ends_with(')') {
            return self.eval_simple_math(&clean[1..clean.len() - 1]);
        }

        // Parse literal number
        clean
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("Cannot parse numeric term in expression: '{clean}'"))
    }

    /// Re-evaluates every calculation across a collection of fields.
    ///
    /// Calculations routinely depend on one another (`Tax = Total * 0.18` while
    /// `Total = Price * Qty`), but `calc_scripts` is a `HashMap`, so a single
    /// pass visits them in an arbitrary order and evaluates a field against
    /// whatever its inputs happened to hold at that instant. Passes are repeated
    /// until the values stop changing, and bounded, so a cyclic set of scripts
    /// settles rather than hanging.
    pub fn recalculate_all(
        &mut self,
        fields: &mut [FormField],
        calc_scripts: &HashMap<String, String>,
    ) {
        /// Generous bound: a dependency chain settles after at most one pass per
        /// link, and no real form nests anywhere near this deep.
        const MAX_PASSES: usize = 16;

        for _ in 0..MAX_PASSES {
            let mut changed = false;
            for (field_name, script) in calc_scripts {
                let Ok(new_val) = self.evaluate_calculation(field_name, script) else {
                    continue;
                };
                let Some(target) = fields.iter_mut().find(|f| f.name == *field_name) else {
                    continue;
                };
                // Every variant is written, not just `Text`. A calculation aimed
                // at a checkbox, radio button or combo box used to be computed
                // and then silently dropped.
                if write_calculated_value(target, &new_val) {
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }
}

/// Writes a calculated value into a form field, in that variant's own terms.
///
/// Returns whether the field actually changed. A checkbox holds a boolean, not
/// a string, so the calculated text has to be interpreted rather than stored:
/// writing it straight into the variant was a type error, and the previous code
/// quietly skipped every non-`Text` target instead.
fn write_calculated_value(field: &mut FormField, value: &str) -> bool {
    // Reuses the engine's own notion of truth rather than inventing a second
    // one; a calculated value of `""`, `"0"` or `"Off"` means unchecked.
    let truthy = FormFieldValue::Text(value.to_string()).is_truthy();

    match &mut field.variant {
        FormFieldVariant::Text { value: current } => {
            if *current == value {
                return false;
            }
            *current = value.to_string();
            true
        }
        FormFieldVariant::Checkbox { is_checked, .. } => {
            if *is_checked == truthy {
                return false;
            }
            *is_checked = truthy;
            true
        }
        FormFieldVariant::RadioButton { is_selected, .. } => {
            if *is_selected == truthy {
                return false;
            }
            *is_selected = truthy;
            true
        }
        FormFieldVariant::ComboBox {
            selected_index,
            export_values,
            options,
        } => {
            // A calculated value names the option's *export* value; fall back to
            // the label, and to clearing the selection when neither matches.
            let idx = export_values
                .iter()
                .position(|v| v == value)
                .or_else(|| options.iter().position(|v| v == value));
            if *selected_index == idx {
                return false;
            }
            *selected_index = idx;
            true
        }
    }
}
