#![cfg(feature = "test-bpf")]

use program_test::GovernanceChatProgramTest;
use solana_program_test::tokio;

mod program_test;

#[tokio::test]
async fn test_post_message() {
    // Arrange
    let mut governance_chat_test = GovernanceChatProgramTest::start_new().await;

    let proposal_cookie = governance_chat_test.with_proposal().await;

    // Act
    let chat_message_cookie = governance_chat_test
        .with_chat_message(&proposal_cookie)
        .await;

    // Assert
    let chat_message_data = governance_chat_test
        .get_message_account(&chat_message_cookie.address)
        .await;

    assert_eq!(chat_message_data, chat_message_cookie.account);
}
