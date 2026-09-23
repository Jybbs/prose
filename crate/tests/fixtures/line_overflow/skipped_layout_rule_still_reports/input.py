from package.module import alpha_name, beta_name, delta_name, gamma_name, lambda_name, omega_name, sigma_name, theta_name  # prose: skip[reflow-imports]

TOTAL = combine(first_argument_value, second_argument_value, third_argument_value, fourth)  # prose: skip[reflow-calls]

TABLE = {"alpha": alpha_value, "beta": beta_value, "delta": delta_value, "gamma": gamma_value}  # prose: skip[reflow-collections]

RESULT = combine(
    first_argument_value_long_name, second_argument_value_long_name, third_argument_valux,
)  # prose: skip[reflow-calls]


def configure(first_parameter, second_parameter, third_parameter, fourth_parameter_value):  # prose: skip[reflow-signatures]
    return first_parameter


def describe():
    """Describe the configuration this module carries, in a sentence long enough to overflow."""  # prose: skip[wrap-docstrings]


def locate():
    """https://example.com/a/deeply/nested/path/that/runs/well/past/the/docstring/budget/index"""  # prose: skip[wrap-docstrings]


MESSAGE = "the first part of the message runs long, " "and the second part carries it past the budget"  # prose: skip[stack-adjacent-strings]


def dispatch(command):
    match command:
        case "halt": return first_identifier_long_enough_to_matter + second_identifier_long  # prose: skip[align-match-case]
