#|======================================================================
PACKAGE.LISP

1. Single package :xgamengine exporting the engine's public API.
2. Called by: ASDF via xgamengine.asd → all downstream modules.
3. (2025-06-13) Initial creation for MVP Phase 0.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(defpackage :xgamengine
  (:use :cl)
  ;; --- LLM Client ---
  (:export #:*api-key*
           #:*api-base-url*
           #:*model*
           #:ensure-api-key
           #:call-chat-completion
           #:extract-assistant-content
           #:extract-usage
           #:parse-sse-stream)
  ;; --- Prompt Loader ---
  (:export #:*template-dir*
           #:load-template
           #:render-template)
  ;; --- Game State ---
  (:export #:game-state
           #:make-game-state
           #:game-state-id
           #:game-state-scenario
           #:game-state-round
           #:game-state-realm
           #:game-state-realm-progress
           #:game-state-qi
           #:game-state-max-qi
           #:game-state-techniques
           #:game-state-treasures
           #:game-state-spirit-stones
           #:game-state-sect
           #:game-state-relationships
           #:game-state-flags
           #:serialize-state
           #:deserialize-state
           #:state-to-narrative
           #:make-initial-state
           #:apply-state-change)
  ;; --- Memory ---
  (:export #:conversation-window
           #:make-conversation-window
           #:append-turn!
           #:get-context-messages
           #:truncate-if-needed)
  ;; --- Prompt Builder ---
  (:export #:build-system-prompt
           #:build-messages
           #:generate-suggestions)
  ;; --- Engine ---
  (:export #:start-game
           #:process-input
           #:engine-output
           #:engine-output-narrative
           #:engine-output-state-changes
           #:engine-output-suggestions
           #:engine-output-round
           #:engine-output-usage
           #:save-game
           #:load-game))
