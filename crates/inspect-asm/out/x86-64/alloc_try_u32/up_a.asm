inspect_asm::alloc_try_u32::up_a:
	push r15
	push r14
	push r13
	push r12
	push rbx
	mov r14, rsi
	mov rbx, rdi
	mov r15, qword ptr [rsi]
	mov r12, qword ptr [r15]
	mov rax, qword ptr [r15 + 8]
	sub rax, r12
	mov r13, r12
	cmp rax, 7
	ja .LBB0_2
	mov rdi, r15
.LBB0_0:
	mov rax, qword ptr [rdi + 24]
	test rax, rax
	je .LBB0_1
	lea rcx, [rax + 32]
	mov qword ptr [rax], rcx
	mov qword ptr [r14], rax
	mov r13, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	sub rcx, r13
	mov rdi, rax
	cmp rcx, 8
	jb .LBB0_0
	jmp .LBB0_2
.LBB0_1:
	mov esi, 4
	mov r13, rdx
	mov edx, 8
	call bump_scope::chunk_raw::RawChunk<_,A>::append_for
	mov rdx, r13
	mov qword ptr [r14], rax
	mov r13, qword ptr [rax]
.LBB0_2:
	call rdx
	mov dword ptr [r13], eax
	mov dword ptr [r13 + 4], edx
	test eax, eax
	je .LBB0_3
	mov qword ptr [r15], r12
	mov qword ptr [r14], r15
	mov dword ptr [rbx + 4], edx
	mov eax, 1
	jmp .LBB0_4
.LBB0_3:
	lea rax, [r13 + 4]
	add r13, 11
	and r13, -4
	mov rcx, qword ptr [r14]
	mov qword ptr [rcx], r13
	mov qword ptr [rbx + 8], rax
	xor eax, eax
.LBB0_4:
	mov dword ptr [rbx], eax
	mov rax, rbx
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	ret
