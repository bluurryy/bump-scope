inspect_asm::vec_map::grow:
	push r15
	push r14
	push r13
	push r12
	push rbx
	mov r14, rdi
	mov rbx, qword ptr [rsi + 24]
	mov r12, qword ptr [rsi]
	mov r15, qword ptr [rsi + 8]
	mov r13, qword ptr [rsi + 16]
	test r15, r15
	je .LBB0_0
	mov rax, r15
	shr rax, 60
	jne .LBB0_10
	lea rcx, [8*r15]
	mov rdx, qword ptr [rbx]
	mov rax, qword ptr [rdx]
	mov rsi, qword ptr [rdx + 8]
	add rax, 7
	and rax, -8
	sub rsi, rax
	cmp rcx, rsi
	jbe .LBB0_1
	mov rdi, rbx
	mov rsi, r15
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	jmp .LBB0_2
.LBB0_0:
	mov eax, 8
	xor edx, edx
	jmp .LBB0_8
.LBB0_1:
	add rcx, rax
	mov qword ptr [rdx], rcx
.LBB0_2:
	movabs rdx, 4611686018427387903
	lea rcx, [r15 - 1]
	mov rdi, rcx
	and rdi, rdx
	cmp rdi, rcx
	cmovae rdi, rcx
	cmp rdi, 18
	jbe .LBB0_3
	lea rsi, [r15 + rdx]
	and rsi, rdx
	cmp rsi, rcx
	cmovae rsi, rcx
	lea rdx, [r12 + 4*rsi]
	add rdx, 4
	cmp rax, rdx
	jae .LBB0_6
	lea rdx, [rax + 8*rsi]
	add rdx, 8
	cmp r12, rdx
	jae .LBB0_6
.LBB0_3:
	xor edx, edx
	mov rsi, r12
.LBB0_4:
	lea rdi, [r12 + 4*r15]
.LBB0_5:
	mov r8, rdx
	mov edx, dword ptr [rsi]
	mov qword ptr [rax + 8*r8], rdx
	lea rdx, [r8 + 1]
	cmp rcx, r8
	je .LBB0_8
	add rsi, 4
	cmp rsi, rdi
	jne .LBB0_5
	jmp .LBB0_8
.LBB0_6:
	inc rdi
	movabs rdx, 9223372036854775804
	and rdx, rdi
	lea rsi, [r12 + 4*rdx]
	xor r8d, r8d
	xorps xmm0, xmm0
.LBB0_7:
	movsd xmm1, qword ptr [r12 + 4*r8]
	movsd xmm2, qword ptr [r12 + 4*r8 + 8]
	unpcklps xmm1, xmm0
	unpcklps xmm2, xmm0
	movups xmmword ptr [rax + 8*r8], xmm1
	movups xmmword ptr [rax + 8*r8 + 16], xmm2
	add r8, 4
	cmp rdx, r8
	jne .LBB0_7
	cmp rdi, rdx
	jne .LBB0_4
.LBB0_8:
	lea rsi, [r12 + 4*r13]
	mov rcx, qword ptr [rbx]
	cmp rsi, qword ptr [rcx]
	jne .LBB0_9
	mov qword ptr [rcx], r12
.LBB0_9:
	mov qword ptr [r14], rax
	mov qword ptr [r14 + 8], rdx
	mov qword ptr [r14 + 16], rdx
	mov qword ptr [r14 + 24], rbx
	mov rax, r14
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	ret
.LBB0_10:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	ud2
	lea rdx, [r12 + 4*r13]
	mov rcx, qword ptr [rbx]
	cmp rdx, qword ptr [rcx]
	jne .LBB0_11
	mov qword ptr [rcx], r12
.LBB0_11:
	mov rdi, rax
	call _Unwind_Resume@PLT
