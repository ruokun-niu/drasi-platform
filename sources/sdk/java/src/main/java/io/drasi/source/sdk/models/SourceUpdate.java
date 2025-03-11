/*
 * Copyright 2024 The Drasi Authors.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package io.drasi.source.sdk.models;

import com.fasterxml.jackson.databind.JsonNode;

import java.util.List;
import java.util.Map;

/**
 * Represents an update operation in the source database, of either a node or relation.
 */
public class SourceUpdate extends SourceChange {

    @Override
    protected Op getOp() {
        return Op.UPDATE;
    }

    /**
     * Create an update operation of a node with the given id, timestamp, properties, metadata and labels.
     *
     * @param id         The id of the element.
     * @param tsNS       The timestamp of the event in nanoseconds.
     * @param properties The properties of the element.
     * @param metadata   The metadata of the element.
     * @param labels     The labels of the element.
     * @param sourceTsMS The timestamp of the event in the source database in milliseconds.
     * @param sequenceNumber The sequence number of the event in the source database.
     */
    public SourceUpdate(String id, long tsNS, JsonNode properties, Map<String, Object> metadata, List<String> labels, long sourceTsMS, long sequenceNumber) {
        super(id, tsNS, properties, metadata, labels, sourceTsMS, sequenceNumber);

    }

    /**
     * Create an update operation of a relation with the given id, timestamp, properties, metadata, labels, startId and endId.
     *
     * @param id         The id of the element.
     * @param tsNS       The timestamp of the event in nanoseconds.
     * @param properties The properties of the element.
     * @param metadata   The metadata of the element.
     * @param labels     The labels of the element.
     * @param sourceTsMS The timestamp of the event in the source database in milliseconds.
     * @param sequenceNumber The sequence number of the event in the source database.
     * @param startId    The id of the start node.
     * @param endId      The id of the end node.
     */
    public SourceUpdate(String id, long tsNS, JsonNode properties, Map<String, Object> metadata, List<String> labels, long sourceTsMS, long sequenceNumber, String startId, String endId) {
        super(id, tsNS, properties, metadata, labels, sourceTsMS, sequenceNumber, startId, endId);
    }
}
