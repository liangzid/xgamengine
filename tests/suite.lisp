#|======================================================================
SUITE.LISP

1. FiveAM test suite for xgamengine.
2. Run with: (asdf:test-system :xgamengine) or (5am:run! :xgamengine)
3. (2025-06-13) Phase 0+3: system loading, state serialization, memory, template tests.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :cl-user)

(defpackage :xgamengine-tests
  (:use :cl :fiveam)
  (:export #:run-tests))

(in-package :xgamengine-tests)

(def-suite :xgamengine
  :description "Master test suite for xgamengine.")

(in-suite :xgamengine)

;; ---- Phase 0: System Loading ----

(test system-loads
  "Verify that the ASDF system loads without errors."
  (is (asdf:find-system :xgamengine nil))
  (pass))

;; ---- Phase 3: Game State ----

(test test-make-initial-state
  "Verify that make-initial-state creates a valid game-state."
  (let ((state (xgamengine:make-initial-state :scenario "qingyun"
                                              :player-name "TestPlayer")))
    (is (string= "qingyun" (xgamengine:game-state-scenario state)))
    (is (string= "练气期初期" (xgamengine:game-state-realm state)))
    (is (= 100 (xgamengine:game-state-qi state)))
    (is (= 0.0 (xgamengine:game-state-realm-progress state)))
    (is (= 50 (xgamengine:game-state-spirit-stones state)))
    (is (member "青云吐纳术" (xgamengine:game-state-techniques state)
                :test #'string=))))

(test test-state-to-narrative
  "Verify state-to-narrative produces readable Chinese output."
  (let* ((state (xgamengine:make-initial-state))
         (narrative (xgamengine:state-to-narrative state)))
    (is (stringp narrative))
    (is (> (length narrative) 0))
    (is (search "练气期初期" narrative))
    (is (search "青云宗" narrative))
    (is (search "清虚道人" narrative))))

(test test-apply-state-change
  "Verify state changes apply correctly."
  (let ((state (xgamengine:make-initial-state)))
    ;; Test realm progress
    (xgamengine:apply-state-change state '(:realm-progress 0.15))
    (is (= 0.15 (xgamengine:game-state-realm-progress state)))
    ;; Test qi delta
    (xgamengine:apply-state-change state '(:qi-delta -30))
    (is (= 70 (xgamengine:game-state-qi state)))
    ;; Test spirit stones
    (xgamengine:apply-state-change state '(:spirit-stones-delta 20))
    (is (= 70 (xgamengine:game-state-spirit-stones state)))
    ;; Test flag
    (xgamengine:apply-state-change state '(:add-flag "test-flag"))
    (is (member "test-flag" (xgamengine:game-state-flags state)
                :test #'string=))))

(test test-breakthrough
  "Verify that reaching 1.0 realm progress triggers a breakthrough."
  (let ((state (xgamengine:make-initial-state)))
    ;; Add enough progress to trigger breakthrough
    (xgamengine:apply-state-change state '(:realm-progress 1.0))
    (is (string= "练气期中期" (xgamengine:game-state-realm state)))
    (is (= 0.0 (xgamengine:game-state-realm-progress state)))))

(test test-serialize-roundtrip
  "Verify state serialization and deserialization is lossless."
  (let* ((original (xgamengine:make-initial-state :scenario "test"))
         (json-str (xgamengine:serialize-state original))
         (restored (xgamengine:deserialize-state json-str)))
    (is (stringp json-str))
    (is (> (length json-str) 0))
    (is (string= (xgamengine:game-state-scenario original)
                 (xgamengine:game-state-scenario restored)))
    (is (string= (xgamengine:game-state-realm original)
                 (xgamengine:game-state-realm restored)))
    (is (= (xgamengine:game-state-qi original)
           (xgamengine:game-state-qi restored)))))

;; ---- Phase 3: Short-term Memory ----

(test test-conversation-window
  "Verify conversation window append and retrieval."
  (let ((window (xgamengine:make-conversation-window)))
    ;; Append a turn
    (xgamengine:append-turn! window
                             "Hello master"
                             '(:role "assistant" :content "Greetings, disciple."))
    (let ((msgs (xgamengine:get-context-messages window)))
      (is (= 2 (length msgs)))
      (is (string= "user" (getf (first msgs) :role)))
      (is (string= "assistant" (getf (second msgs) :role)))
      (is (string= "Hello master" (getf (first msgs) :content))))))

(test test-window-truncation
  "Verify that conversation window truncates when exceeding max rounds."
  (let ((window (xgamengine:make-conversation-window)))
    ;; Add 20 rounds (user + assistant = 40 messages)
    (dotimes (i 20)
      (xgamengine:append-turn! window
                               (format nil "Turn ~d" i)
                               (list :role "assistant"
                                     :content (format nil "Reply ~d" i))))
    (let ((msgs (xgamengine:get-context-messages window)))
      ;; Should be at most max_history_rounds * 2 + head preservation
      (is (< (length msgs) 40))  ; fewer than original
      (is (> (length msgs) 0)))))  ; not empty

;; ---- Phase 2: Template Loader ----

(test test-render-template
  "Verify template variable interpolation."
  (let ((result (xgamengine:render-template
                 "Hello {{name}}, your realm is {{realm}}."
                 '(("name" . "Disciple") ("realm" . "练气期初期")))))
    (is (string= "Hello Disciple, your realm is 练气期初期." result))))

(test test-render-template-missing-var
  "Verify missing variables are left as-is."
  (let ((result (xgamengine:render-template
                 "Hello {{name}}!"
                 '(("unknown" . "value")))))
    ;; Should still contain the unmatched placeholder
    (is (search "{{name}}" result))))

;; ---- Phase 4: Prompt Builder ----

(test test-generate-suggestions
  "Verify suggestions are generated based on state."
  (let* ((state (xgamengine:make-initial-state))
         (suggestions (xgamengine:generate-suggestions state)))
    (is (= 3 (length suggestions)))
    (is (every #'stringp suggestions))))

;; ---- Test Runner ----

(defun run-tests ()
  "Entry point for running all xgamengine tests."
  (run! :xgamengine))
