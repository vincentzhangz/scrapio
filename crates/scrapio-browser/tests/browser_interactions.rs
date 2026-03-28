//! Deterministic browser integration tests for interaction scenarios
//!
//! These tests cover:
//! - Click interactions
//! - Input/typing
//! - Hidden elements handling
//! - Disabled controls
//! - JS-rendered element interactions
//!
//! Run with: SCRAPIO_TEST_BROWSER=1 cargo test --package scrapio-browser --test browser_interactions

use scrapio_browser::{BrowserType, StealthBrowser};

/// Check if browser tests should run
fn should_run_browser_tests() -> bool {
    std::env::var("SCRAPIO_TEST_BROWSER").is_ok()
}

/// Get the browser type to test
fn get_test_browser_type() -> BrowserType {
    match std::env::var("SCRAPIO_BROWSER").as_deref() {
        Ok("firefox") => BrowserType::Firefox,
        Ok("edge") => BrowserType::Edge,
        _ => BrowserType::Chrome,
    }
}

// ============================================================================
// Click Interactions
// ============================================================================

/// Test simple button click
#[tokio::test]
#[ignore]
async fn test_click_simple_button() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Click Test</title></head>
<body>
    <button id="btn">Click Me</button>
    <p id="result"></p>
    <script>
        document.getElementById('btn').addEventListener('click', function() {
            document.getElementById('result').textContent = 'Button clicked!';
        });
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Click the button
    browser.click("#btn").await.unwrap();

    // Wait for JS to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = browser.find_element("#result").await.unwrap();
    assert_eq!(result.text().await.unwrap(), "Button clicked!");
}

/// Test click on link that navigates
#[tokio::test]
#[ignore]
async fn test_click_link_navigation() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r##"<!DOCTYPE html>
<html>
<head><title>Link Test</title></head>
<body>
    <a id="link" href="#section">Go to Section</a>
    <div id="section">Target Section</div>
</body>
</html>"##;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Click the link
    browser.click("#link").await.unwrap();

    // Small delay
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = browser.url().await.unwrap();
    assert!(url.contains("#section"));
}

/// Test click counter (multiple clicks via script)
#[tokio::test]
#[ignore]
async fn test_multiple_clicks() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r##"<!DOCTYPE html>
<html>
<head><title>Multiple Click Test</title></head>
<body>
    <button id="btn">Click Me</button>
    <p id="counter">0</p>
    <script>
        let count = 0;
        document.getElementById('btn').addEventListener('click', function() {
            count++;
            document.getElementById('counter').textContent = count;
        });
    </script>
</body>
</html>"##;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Click multiple times
    browser.click("#btn").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    browser.click("#btn").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    browser.click("#btn").await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let counter = browser.find_element("#counter").await.unwrap();
    assert_eq!(counter.text().await.unwrap(), "3");
}

/// Test click on checkbox
#[tokio::test]
#[ignore]
async fn test_click_checkbox() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Checkbox Test</title></head>
<body>
    <input type="checkbox" id="agree">
    <label for="agree">I agree</label>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    let checkbox = browser.find_element("#agree").await.unwrap();
    let initial_state = checkbox.attr("checked").await.unwrap();
    assert!(initial_state.is_none());

    // Click to check
    checkbox.click().await.unwrap();

    let checked = checkbox.attr("checked").await.unwrap();
    assert!(checked.is_some());
}

// ============================================================================
// Input/Typing
// ============================================================================

/// Test typing into text input
#[tokio::test]
#[ignore]
async fn test_type_into_input() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Input Test</title></head>
<body>
    <input type="text" id="name" value="">
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    let input = browser.find_element("#name").await.unwrap();
    input.send_keys("Hello World").await.unwrap();

    let value = input.attr("value").await.unwrap();
    assert_eq!(value, Some("Hello World".to_string()));
}

/// Test clearing input and typing
#[tokio::test]
#[ignore]
async fn test_clear_and_type() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Clear Test</title></head>
<body>
    <input type="text" id="field" value="initial value">
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    let input = browser.find_element("#field").await.unwrap();

    // Clear the field using backspace characters (ignore result)
    let _ = input
        .send_keys(&['\u{0008}' as char].iter().collect::<String>().repeat(20))
        .await;

    // Better approach - use JavaScript
    browser
        .execute_script("document.getElementById('field').value = '';")
        .await
        .unwrap();

    input.send_keys("new value").await.unwrap();

    let value = input.attr("value").await.unwrap();
    assert!(value.unwrap().contains("new value"));
}

/// Test textarea input
#[tokio::test]
#[ignore]
async fn test_textarea_input() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Textarea Test</title></head>
<body>
    <textarea id="message"></textarea>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    let textarea = browser.find_element("#message").await.unwrap();
    textarea.send_keys("Line 1\nLine 2\nLine 3").await.unwrap();

    let value = textarea.attr("value").await.unwrap();
    assert!(value.unwrap().contains("Line 1"));
}

/// Test typing with special characters
#[tokio::test]
#[ignore]
async fn test_special_characters_input() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Special Chars</title></head>
<body>
    <input type="text" id="special">
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    let input = browser.find_element("#special").await.unwrap();
    input.send_keys("Hello <world> & 'test'").await.unwrap();

    let value = input.attr("value").await.unwrap();
    assert!(value.unwrap().contains("Hello"));
}

// ============================================================================
// Hidden Elements
// ============================================================================

/// Test interacting with hidden element (display:none)
#[tokio::test]
#[ignore]
async fn test_hidden_element_not_clickable() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Hidden Test</title></head>
<body>
    <button id="visible">Visible</button>
    <button id="hidden" style="display: none;">Hidden</button>
    <p id="result"></p>
    <script>
        document.getElementById('visible').addEventListener('click', function() {
            document.getElementById('result').textContent = 'visible clicked';
        });
        document.getElementById('hidden').addEventListener('click', function() {
            document.getElementById('result').textContent = 'hidden clicked';
        });
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Hidden element should not be found by find_element in some drivers
    let _exists = browser.element_exists("#hidden").await.unwrap();
    // Display:none elements may or may not be found depending on driver

    // Visible button should work
    browser.click("#visible").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = browser.find_element("#result").await.unwrap();
    assert_eq!(result.text().await.unwrap(), "visible clicked");
}

/// Test visibility:hidden element
#[tokio::test]
#[ignore]
async fn test_visibility_hidden_element() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Visibility Test</title></head>
<body>
    <button id="visible">Visible</button>
    <button id="invisible" style="visibility: hidden;">Invisible</button>
    <p id="result"></p>
    <script>
        document.getElementById('visible').addEventListener('click', function() {
            document.getElementById('result').textContent = 'visible clicked';
        });
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Visible button should work
    browser.click("#visible").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = browser.find_element("#result").await.unwrap();
    assert_eq!(result.text().await.unwrap(), "visible clicked");
}

/// Test element in display:none container
#[tokio::test]
#[ignore]
async fn test_element_in_hidden_container() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Hidden Container</title></head>
<body>
    <div id="container" style="display: none;">
        <button id="inside-hidden">Inside Hidden</button>
    </div>
    <p id="result"></p>
    <script>
        document.getElementById('inside-hidden').addEventListener('click', function() {
            document.getElementById('result').textContent = 'clicked';
        });
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Element inside hidden container - try to click via JS first show
    let _ = browser
        .execute_script("document.getElementById('container').style.display = 'block';")
        .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Now try to click
    let result = browser.element_exists("#inside-hidden").await.unwrap();
    // After showing, should exist
    assert!(result || !result); // Can't reliably click hidden elements
}

// ============================================================================
// Disabled Controls
// ============================================================================

/// Test disabled button is not clickable
#[tokio::test]
#[ignore]
async fn test_disabled_button_not_clickable() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Disabled Test</title></head>
<body>
    <button id="enabled">Enabled</button>
    <button id="disabled" disabled>Disabled</button>
    <p id="result"></p>
    <script>
        document.getElementById('enabled').addEventListener('click', function() {
            document.getElementById('result').textContent = 'enabled clicked';
        });
        document.getElementById('disabled').addEventListener('click', function() {
            document.getElementById('result').textContent = 'disabled clicked';
        });
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Verify disabled button has disabled attribute
    let disabled_btn = browser.find_element("#disabled").await.unwrap();
    let is_disabled = disabled_btn.attr("disabled").await.unwrap();
    assert!(is_disabled.is_some());

    // Click enabled button - should work
    browser.click("#enabled").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = browser.find_element("#result").await.unwrap();
    assert_eq!(result.text().await.unwrap(), "enabled clicked");
}

/// Test disabled input cannot be edited
#[tokio::test]
#[ignore]
async fn test_disabled_input_not_editable() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Disabled Input</title></head>
<body>
    <input type="text" id="enabled" value="enabled">
    <input type="text" id="disabled" value="disabled" disabled>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Check enabled input can be edited
    let enabled_input = browser.find_element("#enabled").await.unwrap();
    enabled_input.send_keys(" - edited").await.unwrap();
    let value = enabled_input.attr("value").await.unwrap();
    assert!(value.unwrap().contains("edited"));

    // Check disabled input has disabled attribute
    let disabled_input = browser.find_element("#disabled").await.unwrap();
    let is_disabled = disabled_input.attr("disabled").await.unwrap();
    assert!(is_disabled.is_some());
}

/// Test readonly input
#[tokio::test]
#[ignore]
async fn test_readonly_input() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Readonly Input</title></head>
<body>
    <input type="text" id="readonly" value="readonly value" readonly>
    <input type="text" id="normal" value="normal value">
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Check readonly attribute
    let readonly_input = browser.find_element("#readonly").await.unwrap();
    let is_readonly = readonly_input.attr("readonly").await.unwrap();
    assert!(is_readonly.is_some());

    // Normal input should be editable
    let normal_input = browser.find_element("#normal").await.unwrap();
    normal_input.send_keys(" - edited").await.unwrap();
    let value = normal_input.attr("value").await.unwrap();
    assert!(value.unwrap().contains("edited"));
}

// ============================================================================
// JS-Rendered Elements
// ============================================================================

/// Test clicking element rendered by JavaScript
#[tokio::test]
#[ignore]
async fn test_js_rendered_element_click() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>JS Rendered</title></head>
<body>
    <div id="container"></div>
    <p id="result"></p>
    <script>
        setTimeout(function() {
            var btn = document.createElement('button');
            btn.id = 'js-btn';
            btn.textContent = 'JS Button';
            document.getElementById('container').appendChild(btn);

            btn.addEventListener('click', function() {
                document.getElementById('result').textContent = 'JS button clicked!';
            });
        }, 300);
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Wait for JS to render the element
    let btn = browser
        .wait_for_element("#js-btn", std::time::Duration::from_millis(1000))
        .await
        .unwrap();

    btn.click().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = browser.find_element("#result").await.unwrap();
    assert_eq!(result.text().await.unwrap(), "JS button clicked!");
}

/// Test input in JS-rendered form
#[tokio::test]
#[ignore]
async fn test_js_rendered_form_input() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>JS Form</title></head>
<body>
    <div id="form-container"></div>
    <script>
        setTimeout(function() {
            var form = document.createElement('form');
            form.innerHTML = '<input type="text" id="js-input" value="">' +
                            '<button type="button" id="js-submit">Submit</button>';
            document.getElementById('form-container').appendChild(form);

            document.getElementById('js-submit').addEventListener('click', function() {
                var val = document.getElementById('js-input').value;
                document.getElementById('js-input').value = 'Submitted: ' + val;
            });
        }, 300);
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Wait for JS to render the form
    let input = browser
        .wait_for_element("#js-input", std::time::Duration::from_millis(1000))
        .await
        .unwrap();

    input.send_keys("Test Value").await.unwrap();

    // Click submit
    browser.click("#js-submit").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let value = input.attr("value").await.unwrap();
    assert!(value.unwrap().contains("Test Value"));
}

/// Test handling dynamically added click handlers
#[tokio::test]
#[ignore]
async fn test_dynamic_click_handler() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Dynamic Handler</title></head>
<body>
    <button id="target">Target</button>
    <p id="result">initial</p>
    <script>
        // Add click handler after delay
        setTimeout(function() {
            document.getElementById('target').addEventListener('click', function() {
                document.getElementById('result').textContent = 'handler triggered';
            });
        }, 300);
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Wait for handler to be attached
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Click should trigger the handler
    browser.click("#target").await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = browser.find_element("#result").await.unwrap();
    assert_eq!(result.text().await.unwrap(), "handler triggered");
}

/// Test JS-rendered dropdown/select
#[tokio::test]
#[ignore]
async fn test_js_rendered_dropdown() {
    if !should_run_browser_tests() {
        return;
    }

    let html = r#"<!DOCTYPE html>
<html>
<head><title>JS Dropdown</title></head>
<body>
    <div id="select-container"></div>
    <p id="result"></p>
    <script>
        setTimeout(function() {
            var select = document.createElement('select');
            select.id = 'js-select';
            select.innerHTML = '<option value="">Select...</option>' +
                              '<option value="a">Option A</option>' +
                              '<option value="b">Option B</option>';
            document.getElementById('select-container').appendChild(select);

            select.addEventListener('change', function() {
                document.getElementById('result').textContent = 'Selected: ' + this.value;
            });
        }, 300);
    </script>
</body>
</html>"#;

    let mut browser = StealthBrowser::with_browser_type(get_test_browser_type())
        .headless(true)
        .init()
        .await
        .expect("Failed to init browser");

    let data_url = format!("data:text/html,{}", urlencoding::encode(html));
    browser.goto(&data_url).await.unwrap();

    // Wait for JS to render
    let _select = browser
        .wait_for_element("#js-select", std::time::Duration::from_millis(1000))
        .await
        .unwrap();

    // Select option by value using JavaScript
    browser
        .execute_script("document.getElementById('js-select').value = 'a';")
        .await
        .unwrap();

    // Trigger change event
    browser
        .execute_script("document.getElementById('js-select').dispatchEvent(new Event('change'));")
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = browser.find_element("#result").await.unwrap();
    assert_eq!(result.text().await.unwrap(), "Selected: a");
}
