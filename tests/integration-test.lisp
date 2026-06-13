#|======================================================================
INTEGRATION-TEST.LISP

1. Integration tests for xgamengine — tests that require network and API access.
2. Run selectively: (5am:run! :xgamengine-integration)
3. (2025-06-13) Phase 7: API connectivity and end-to-end tests.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine-tests)

(def-suite :xgamengine-integration
  :description "Integration tests requiring network and API access."
  :in :xgamengine)

(in-suite :xgamengine-integration)

(test test-llm-connectivity
  "Verify that the LLM API is reachable and returns a valid response.
Requires DEEPSEEK_API_KEY to be set."
  ;; Simple connectivity test — send a minimal message
  (let* ((messages (list (list :role "user" :content "Hello")))
         (response (xgamengine:call-chat-completion messages
                                                    :temperature 0.0
                                                    :max-tokens 32)))
    (is (not (null response)))
    (let ((content (xgamengine:extract-assistant-content response)))
      (is (stringp content))
      (is (> (length content) 0))
      (format t "~%LLM Response: ~a~%" content))))

(test test-llm-chinese
  "Verify that the LLM can respond in Chinese (critical for xianxia game)."
  (let* ((messages (list (list :role "user"
                               :content "请用中文回答：你好")))
         (response (xgamengine:call-chat-completion messages
                                                    :temperature 0.0
                                                    :max-tokens 64)))
    (let ((content (xgamengine:extract-assistant-content response)))
      (is (stringp content))
      ;; Check for Chinese characters
      (is (some (lambda (c) (> (char-code c) 127))
                (coerce content 'list)))
      (format t "~%Chinese Response: ~a~%" content))))

(test test-api-key-loading
  "Verify API key can be loaded from file or environment."
  (let ((key (xgamengine:ensure-api-key)))
    (is (stringp key))
    (is (> (length key) 10))))

(test test-template-loading
  "Verify prompt templates can be loaded from disk."
  (let ((content (xgamengine:load-template "world-rules")))
    (is (stringp content))
    (is (> (length content) 100))
    (is (search "修仙世界" content))
    (is (search "{{state-narrative}}" content))))

(test test-full-game-round
  "End-to-end test: start a game and process one input.
This is the smoke test for the entire engine."
  (let ((output (xgamengine:start-game :scenario "qingyun"
                                       :player-name "TestPlayer")))
    (is (not (null output)))
    (let ((narrative (xgamengine:engine-output-narrative output)))
      (is (stringp narrative))
      (is (> (length narrative) 10))
      (format t "~%Opening Narrative: ~a~%" narrative))

    ;; Process one turn
    (let ((output2 (xgamengine:process-input "我想修炼")))
      (is (not (null output2)))
      (let ((narrative2 (xgamengine:engine-output-narrative output2)))
        (is (stringp narrative2))
        (is (> (length narrative2) 10))
        (format t "~%Turn 1 Response: ~a~%" narrative2))
      ;; Check suggestions
      (let ((suggestions (xgamengine:engine-output-suggestions output2)))
        (is (= 3 (length suggestions)))))))

(defun run-integration-tests ()
  "Run integration tests only."
  (run! :xgamengine-integration))
