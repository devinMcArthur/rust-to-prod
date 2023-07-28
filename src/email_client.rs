use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

pub struct EmailClient {
    http_client: Client,
    base_url: reqwest::Url,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: reqwest::Url,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = self
            .base_url
            .join("/mail/send")
            .expect("Could not create email client URL");
        let request_body = SendEmailRequest {
            personalizations: vec![SendEmailPersonalizations {
                to: SendEmailObject {
                    email: recipient.as_ref(),
                    name: None,
                },
            }],
            from: SendEmailObject {
                email: self.sender.as_ref(),
                name: None,
            },
            subject,
            content: vec![
                SendEmailContent {
                    r#type: SendGridContentType::Plain,
                    value: text_content,
                },
                SendEmailContent {
                    r#type: SendGridContentType::Html,
                    value: html_content,
                },
            ],
        };

        self.http_client
            .post(url)
            .header(
                "Authorization",
                format!("Bearer: {}", self.authorization_token.expose_secret()),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
struct SendEmailRequest<'a> {
    personalizations: Vec<SendEmailPersonalizations<'a>>,
    from: SendEmailObject<'a>,
    subject: &'a str,
    content: Vec<SendEmailContent<'a>>,
}

#[derive(serde::Serialize)]
struct SendEmailObject<'a> {
    email: &'a str,
    name: Option<&'a str>,
}

#[derive(serde::Serialize)]
struct SendEmailPersonalizations<'a> {
    to: SendEmailObject<'a>,
}

#[derive(serde::Serialize)]
struct SendEmailContent<'a> {
    r#type: SendGridContentType,
    value: &'a str,
}

#[derive(serde::Serialize)]
enum SendGridContentType {
    Plain,
    Html,
}

#[allow(dead_code)]
impl SendGridContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Plain => "text/plain",
            Self::Html => "text/html",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use super::SendGridContentType;

    // Used to check the body sent by `send_email`
    struct SendEmailBodyMatcher;
    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            // Try to parse the body as a JSON value
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                let personalizations_valid = body
                    .get("personalizations")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(0))
                    .and_then(|v| {
                        v.get("to")
                            .and_then(|to| to.get("email").and_then(|_| Some(true)))
                    })
                    .unwrap_or(false);

                // Check 'from' field is an object and 'email' field exists in it
                let from_valid = body
                    .get("from")
                    .and_then(|v| v.as_object())
                    .and_then(|o| o.get("email").and_then(|_| Some(true)))
                    .unwrap_or(false);

                // Check 'subject' field exists and is a string
                let subject_valid = body.get("subject").and_then(|v| v.as_str()).is_some();

                // Check 'content' field is an array and 'type' & 'value' fields exist in its first object
                let content_valid = body
                    .get("content")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(0))
                    .and_then(|v| {
                        Some(
                            v.get("type")
                                .and_then(|t| {
                                    t.as_str().map(|s| {
                                        s == SendGridContentType::Plain.as_str()
                                            || s == SendGridContentType::Html.as_str()
                                    })
                                })
                                .is_some()
                                && v.get("value").and_then(|_| Some(true)).is_some(),
                        )
                    })
                    .unwrap_or(false);

                personalizations_valid && from_valid && subject_valid && content_valid
            } else {
                false
            }
        }
    }

    /// Generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Generate random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Generate a random SubscriberEmail
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    /// Get a test instance of `EmailClient`
    fn email_client(base_url: String) -> EmailClient {
        let parsed_base_url = reqwest::Url::parse(base_url.as_str()).unwrap();
        EmailClient::new(
            parsed_base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // Defines the behaviour of the mounted server
        Mock::given(header_exists("Authorization")) // Matches all incoming requests
            .and(header("Content-Type", "application/json"))
            .and(path("/mail/send"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200)) // Always returns a `200`
            .expect(1) // Should only match once
            .mount(&mock_server) // Mount `MockerServer`
            .await;

        // Act
        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // We do not copy in all the matchers we have in the other test.
        // The purpose of this test is not to assert on the request we
        // are sending out.
        // We add the bare minimum needed to trigger the path we want
        // to test in `send_email`.
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }
}
