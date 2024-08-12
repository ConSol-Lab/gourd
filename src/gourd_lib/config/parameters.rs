use std::collections::BTreeMap;
use std::collections::BTreeSet;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use log::trace;

use super::Parameter;
use super::UserInput;
use crate::bailc;
use crate::constants::INTERNAL_PARAMETER;
use crate::constants::INTERNAL_PREFIX;
use crate::constants::PARAMETER_ESCAPE;
use crate::constants::SUB_PARAMETER_ESCAPE;
use crate::ctx;

/// Check if the parameters are well-formed.
pub fn validate_parameters(parameters: &BTreeMap<String, Parameter>) -> Result<()> {
    for (p_name, p) in parameters {
        if p.sub.is_some() && p.values.is_some() {
            bailc!(
              "Parameter specified incorrectly", ;
              "Parameter can have either values or subparameters, not both", ;
              "Parameter name {}", p_name
            );
        } else if p.sub.is_none() && p.values.is_none() {
            bailc!(
              "Parameter specified incorrectly", ;
              "Parameter must have either values or subparameters, currently has none", ;
              "Parameter name {}", p_name
            );
        }
    }

    Ok(())
}

/// Takes the set of all inputs and all Parameters and expands parameterd
/// arguments in the inputs with valeus of provided parameters.
///
/// # Examples
///
/// ```toml
/// [parameters.x.sub.a]
/// values = ["1", "2"]
///
/// [parameters.x.sub.b]
/// values = ["15", "60"]
///
/// [parameters.y]
/// values = ["a", "b"]
///
/// [programs.test_program]
/// binary = "test"
///
/// [inputs.test_input]
/// arguments = [ "const", "subparam|x.a", "param|y",
/// "parameter|{parameter_x_b}" ]
/// ```
///
/// Will get expanded to:
/// ```toml
/// [inputs.test_input_x_0_y_0]
/// arguments = [ "const", "1", "a", "15" ]
///
/// [inputs.test_input_x_1_y_0]
/// arguments = [ "const", "2", "a", "60" ]
///
/// [inputs.test_input_x_0_y_1]
/// arguments = [ "const", "1", "b", "15" ]
///
/// [inputs.test_input_x_1_y_1]
/// arguments = [ "const", "2", "b", "60" ]
/// ```
pub fn expand_parameters(
    inputs: BTreeMap<String, UserInput>,
    parameters: &BTreeMap<String, Parameter>,
) -> Result<BTreeMap<String, UserInput>> {
    let mut result: BTreeMap<String, UserInput> = BTreeMap::new();

    check_sub_parameter_size_is_equal(parameters)?;

    for (input_name, input) in inputs.iter() {
        let mut map = BTreeMap::new();
        let mut expandable_parameters = BTreeSet::new();

        // Find uses of parameters in inputs.
        get_expandable_parameters(input, &mut map, &mut expandable_parameters)?;

        trace!("Expandable parameters for {input_name} are {expandable_parameters:#?}");

        // If none of parameters was used in this input then there's no need to do
        // anything.
        if expandable_parameters.is_empty() {
            result.insert(input_name.clone(), input.clone());
            continue;
        }

        let mut set: BTreeSet<(String, Vec<String>)> = BTreeSet::new();
        set.insert((input_name.clone(), input.arguments.clone()));

        for parameter_name in expandable_parameters {
            if let Some(param) = parameters.get(&parameter_name) {
                let indexes = &map[&parameter_name];

                if indexes[0].1.is_some() {
                    set = expand_subparameter(&parameter_name, param, set, indexes)?;
                } else {
                    set = expand_parameter(&parameter_name, param, set, indexes)?;
                }
            } else {
                bailc!(
                    "Invalid parameter specified", ;
                    "Did not find values for parameter specified in input {}", input_name;
                    "For parameter \"{parameter_name}\"",
                );
            }
        }

        for (name, x) in set {
            let mut input_copy = input.clone();
            input_copy.arguments.clone_from(&x);
            result.insert(
                format!("{name}{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
                input_copy,
            );
        }
    }

    Ok(result)
}

/// Checks if all subparameters of each paramterers specified in `parameters`
/// are equal (Helper function).
fn check_sub_parameter_size_is_equal(parameters: &BTreeMap<String, Parameter>) -> Result<()> {
    for (parameter_name, parameter) in parameters {
        if let Some(sub_parameters) = &parameter.sub {
            let sub_parameter_size = sub_parameters
                .clone()
                .first_entry()
                .ok_or(anyhow!("There needs to be some subparameters specified"))
                .with_context(ctx!("For parameter {parameter_name}", ; "",))?
                .get()
                .values
                .len();

            for x in sub_parameters.values() {
                if x.values.len() != sub_parameter_size {
                    bailc!(
                        "Subparameter sizes don't match", ;
                        "For parameter: {}", parameter_name;
                        "",
                    );
                }
            }
        }
    }
    Ok(())
}

/// Gets names and positions of parameters in the provided `input`. (Helper
/// function)
///
/// Saves paramter names in `expandable_parameters` Set and
/// `parameter_names_encountered` Set
///
/// Saves map of Index to Parameter name in `map`
fn get_expandable_parameters(
    input: &UserInput,
    map: &mut BTreeMap<String, Vec<(usize, Option<String>)>>,
    expandable_parameters: &mut BTreeSet<String>,
) -> Result<()> {
    /// Ensures that the syntax is correct.
    fn constrict_syntax(inner: Option<&str>) -> Result<&str> {
        inner
            .ok_or(anyhow!("A subparameter requires the syntax param.subparam"))
            .with_context(ctx!("", ; "", ))
    }

    for (pos, arg) in input.arguments.iter().enumerate() {
        if let Some(param_name) = arg.strip_prefix(PARAMETER_ESCAPE).to_owned() {
            expandable_parameters.insert(param_name.to_string());

            if let Some(vector) = map.get_mut(param_name) {
                vector.push((pos, None))
            } else {
                map.insert(param_name.to_string(), vec![(pos, None)]);
            }
        } else if let Some(whole_subparam) = arg.strip_prefix(SUB_PARAMETER_ESCAPE) {
            let mut dot_iter = whole_subparam.split('.');

            let param_name = constrict_syntax(dot_iter.next())?.to_string();

            let subparam_name = constrict_syntax(dot_iter.next())?.to_string();

            if dot_iter.next().is_some() {
                bailc!("Invalid paramter syntax", ; "", ; "",);
            }

            expandable_parameters.insert(param_name.clone());

            if let Some(vector) = map.get_mut(&param_name) {
                vector.push((pos, Some(subparam_name)));
            } else {
                map.insert(param_name, vec![(pos, Some(subparam_name))]);
            }
        }
    }

    Ok(())
}

/// Expands provided parameter (Helper function).
fn expand_parameter(
    parameter_name: &String,
    param: &Parameter,
    set: BTreeSet<(String, Vec<String>)>,
    indexes: &Vec<(usize, Option<String>)>,
) -> Result<BTreeSet<(String, Vec<String>)>> {
    let param_values = &param
        .values
        .clone()
        .ok_or(anyhow!("Parameter \"{parameter_name}\" used"))
        .with_context(ctx!(
            "", ;
            "You cannot use a parameter in a '{PARAMETER_ESCAPE}' while no values are specified",
        ))?;

    let mut new_set = BTreeSet::new();

    for (base_name, arguments) in set {
        // For each value...
        for (i, value) in param_values.iter().enumerate() {
            let mut arguments_clone = arguments.clone();

            // Everywhere where this parameter appears we replace it with
            // its value.
            for index in indexes {
                if index.1.is_some() {
                    bailc!(
                        "Ivariant failed", ;
                        "You cannot use subparameters for this parameter anymore", ;
                        "For parameter \"{parameter_name}\"",
                    );
                }
                arguments_clone[index.0] = value.to_string();
            }

            new_set.insert((format!("{base_name}_{parameter_name}_{i}"), arguments_clone));
        }
    }

    Ok(new_set)
}

/// Expands provided subparameter (Helper function).
fn expand_subparameter(
    param_name: &String,
    param: &Parameter,
    set: BTreeSet<(String, Vec<String>)>,
    indexes: &Vec<(usize, Option<String>)>,
) -> Result<BTreeSet<(String, Vec<String>)>> {
    let subparams = &param
        .sub
        .clone()
        .ok_or(anyhow!("Parameter {param_name} used"))
        .with_context(ctx!(
            "", ;
            "You cannot use a parameter in a '{SUB_PARAMETER_ESCAPE}' while no subparameters are specified",
        ))?;

    let size_of_one = subparams
        .values()
        .next()
        .ok_or(anyhow!("Subparameters required for {param_name}"))
        .context("")?
        .values
        .len();

    let mut new_set = BTreeSet::new();

    for (base_name, arguments) in set {
        for i in 0..size_of_one {
            let mut arguments_clone = arguments.clone();
            for sub_index in indexes {
                let expanding = &sub_index
                    .1
                    .clone()
                    .ok_or(anyhow!("Invariant failed"))
                    .with_context(ctx!(
                        "Cannot use a parameter with subparameters as value-based", ;
                        "'{INTERNAL_PARAMETER}' reqiures a parameter that has the values \
                        field defined and \"{param_name}\" does not",
                    ))?;

                arguments_clone[sub_index.0].clone_from(
                    &subparams
                        .get(expanding)
                        .ok_or(anyhow!("Invalid subparameter specified {expanding}"))
                        .with_context(ctx!(
                        "For parameter {param_name}", ;
                        "Ensure that it exists", ))?
                        .values[i],
                );
            }

            new_set.insert((format!("{base_name}_{param_name}_{i}"), arguments_clone));
        }
    }

    Ok(new_set)
}

#[cfg(test)]
#[path = "tests/parameters.rs"]
mod tests;
