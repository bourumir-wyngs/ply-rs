import struct


def write_ply(filename, vertex_format, index_format, vertices, face):
    """
    Writes a PLY file with the given vertex and face formats.

    Args:
        filename (str): Name of the output file.
        vertex_format (str): Format string for vertex coordinates.
        index_format (str): Format string for face indices.
        vertices (list): List of vertices (tuples of coordinates).
        face (list): Face data (count followed by vertex indices).
    """
    # Map formats to PLY property declarations
    vertex_properties = {
        '<fff': 'property float x\nproperty float y\nproperty float z\n',
        '<ddd': 'property double x\nproperty double y\nproperty double z\n',
    }
    index_properties = {
        '<Biii': 'property list uchar int vertex_indices\n',
        '<Bhhh': 'property list uchar short vertex_indices\n',
    }

    # Create the PLY header
    header = f"""ply
format binary_little_endian 1.0
element vertex {len(vertices)}
{vertex_properties[vertex_format]}element face 1
{index_properties[index_format]}end_header
"""

    with open(filename, 'wb') as file:
        # Write header
        file.write(header.encode('ascii'))
        # Write vertex data
        for vertex in vertices:
            file.write(struct.pack(vertex_format, *vertex))
        # Write face data
        file.write(struct.pack('<B', face[0]))  # Face count as uchar
        face_indices_format = index_format[2:]  # Extract the index part of the format
        file.write(struct.pack(face_indices_format, *face[1:]))


# This defines the rotated, right triangle with edges 3:4:5
vertices = [
    (-0.7, 2.1, 0),  # Rotated Vertex 1
    (1.4, 4.2, 0),  # Rotated Vertex 2
    (-3.5, 4.9, 0),  # Rotated Vertex 3
]
face = [3, 0, 1, 2]  # A single triangular face

# Generate files with various combinations
write_ply('../example_plys/diverse_field_formats/floats_ints.ply', '<fff', '<Biii', vertices, face)  # Float vertices, Int indices
write_ply('../example_plys/diverse_field_formats/floats_shorts.ply', '<fff', '<Bhhh', vertices, face)  # Float vertices, Short indices
write_ply('../example_plys/diverse_field_formats/doubles_ints.ply', '<ddd', '<Biii', vertices, face)  # Double vertices, Int indices
write_ply('../example_plys/diverse_field_formats/doubles_shorts.ply', '<ddd', '<Bhhh', vertices, face)  # Double vertices, Short indices
