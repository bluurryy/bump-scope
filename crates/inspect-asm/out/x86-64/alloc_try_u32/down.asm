inspect_asm::alloc_try_u32::down:
	push r15
	push r14
	push r13
	push r12
	push rbx
	mov r14, rsi
	mov rbx, rdi
	mov r15, qword ptr [rsi]
	mov r12, qword ptr [r15]
	mov r13, r12
	and r13, -4
	add r13, -8
	setne al
	cmp r13, qword ptr [r15 + 8]
	setae cl
	test cl, al
	jne .LBB0_2
	mov rdi, r15
.LBB0_0:
	mov rax, qword ptr [rdi + 24]
	test rax, rax
	je .LBB0_1
	mov qword ptr [rax], rax
	mov qword ptr [r14], rax
	mov r13, qword ptr [rax]
	and r13, -4
	add r13, -8
	sete cl
	cmp r13, qword ptr [rax + 8]
	setb sil
	or sil, cl
	mov rdi, rax
	jne .LBB0_0
	jmp .LBB0_2
.LBB0_1:
	mov esi, 4
	mov r13, rdx
	mov edx, 8
	call bump_scope::chunk_raw::RawChunk<_,A>::append_for
	mov rdx, r13
	mov qword ptr [r14], rax
	mov r13, qword ptr [rax]
	and r13, -4
	add r13, -8
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
	add r13, 4
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r13
	mov qword ptr [rbx + 8], r13
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
